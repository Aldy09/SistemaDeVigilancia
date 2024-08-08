use crate::mqtt::messages::connect_message::ConnectMessage;
use crate::mqtt::messages::{
    disconnect_message::DisconnectMessage, puback_message::PubAckMessage,
    publish_message::PublishMessage, suback_message::SubAckMessage,
    subscribe_message::SubscribeMessage, subscribe_return_code::SubscribeReturnCode,
};

use crate::mqtt::server::user::User;
use std::net::{TcpListener, TcpStream};
use std::thread;

use crate::mqtt::server::user_state::UserState;
use std::collections::hash_map::ValuesMut;
use std::collections::{HashMap, VecDeque};
use std::fs::File;

use std::io::{Error, ErrorKind, Write};
use std::sync::{Arc, Mutex};

use super::incoming_connections::ClientListener;

const TOPIC_MESSAGES_LEN: usize = 50;
type ShareableUsers = Arc<Mutex<HashMap<String, User>>>;
type StreamType = TcpStream;
type TopicMessages = VecDeque<PublishMessage>; // Se guardaran todos los mensajes, y se enviaran en caso de reconexión o si un cliente no recibio ciertos mensajes.

fn clean_file(file_path: &str) -> Result<(), Error> {
    let mut file = File::create(file_path)?;
    file.write_all(b"")?; // Escribe un contenido vacío para limpiarlo
    Ok(())
}

#[derive(Debug)]
pub struct MQTTServer {
    connected_users: ShareableUsers,
    available_packet_id: u16,                                      //
    messages_by_topic: Arc<Mutex<HashMap<String, TopicMessages>>>, // String = topic
}

impl MQTTServer {
    pub fn new(ip: String, port: u16) -> Result<Self, Error> {
        let file_path = "log.txt";
        if let Err(e) = clean_file(file_path) {
            println!("Error al limpiar el archivo: {:?}", e);
        }

        let mqtt_server = Self {
            connected_users: Arc::new(Mutex::new(HashMap::new())),
            available_packet_id: 0,
            messages_by_topic: Arc::new(Mutex::new(HashMap::new())),
        };

        let listener = create_server(ip, port)?;
        let mut incoming_connections = ClientListener::new();
        let self_clone = mqtt_server.clone_ref();
        // Hilo para manejar las conexiones entrantes
        let thread_incoming = thread::spawn(move || {
            let _ = incoming_connections.handle_incoming_connections(listener, self_clone);
        });

        thread_incoming.join().unwrap();

        Ok(mqtt_server)
    }

    /// Agrega un PublishMessage a la estructura de mensajes de su topic.
    fn add_message_to_topic_messages(
        &self,
        publish_msg: PublishMessage,
        msgs_by_topic_l: &mut std::sync::MutexGuard<'_, HashMap<String, TopicMessages>>,
    ) {
        let topic = publish_msg.get_topic();

        // Obtiene o crea (si no existía) el VeqDequeue<PublishMessage> correspondiente al topic del publish message
        let topic_messages = msgs_by_topic_l
            .entry(topic)
            //.or_insert_with(VecDeque::new);
            .or_default(); // clippy.
                           // Inserta el PublishMessage en el HashMap interno
        topic_messages.push_back(publish_msg);
    }

    /// Busca al client_id en el hashmap de conectados, si ya existía analiza su estado:
    /// si ya estaba como activo, es un usuario duplicado por lo que le envía disconnect al stream anterior;
    /// si estaba como desconectado temporalmente (ie ctrl+C), se está reconectando.
    /// Devuelve true si era reconexión, false si no era reconexión.
    pub fn manage_possible_reconnecting_or_duplicate_user(
        &self,
        client_id: &str,
        new_stream_of_reconnected_user: &StreamType,
    ) -> Result<bool, Error> {
        if let Ok(mut connected_users_locked) = self.connected_users.lock() {
            if let Some(client) = connected_users_locked.get_mut(client_id) {
                match client.get_state() {
                    UserState::Active => {
                        // El cliente ya se encontraba activo ==> Es duplicado.
                        self.handle_duplicate_user(client)?;
                    }
                    UserState::TemporallyDisconnected => {
                        // El cliente se encontraba temp desconectado ==> Se está reconectando.
                        self.handle_reconnecting_user(client, new_stream_of_reconnected_user)?;
                        // Único caso en que devuelve true.
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// Desconecta al user previo que ya existía, para permitir la conexión con el nuevo.
    fn handle_duplicate_user(&self, client: &mut User) -> Result<(), Error> {
        let msg = DisconnectMessage::new();
        client.write_message(&msg.to_bytes())?;
        Ok(())
    }

    /// Actualiza el stream al nuevo stream que ahora tiene user luego de aberse reconectado; y
    /// le envía por ese nuevo stream a user los mensajes que no recibió por estar desconectado.
    fn handle_reconnecting_user(
        &self,
        client: &mut User,
        new_stream_of_reconnected_user: &StreamType,
    ) -> Result<(), Error> {
        client.set_state(UserState::Active);
        client.update_stream_with(new_stream_of_reconnected_user.try_clone()?);

        // Envía los mensajes que no recibió de todos los topics a los que está suscripto
        let topics = client.get_topics().to_vec();
        for topic in topics {
            // Necesitamos los mensajes
            if let Ok(mut messages_by_topic_locked) = self.messages_by_topic.lock() {
                if let Some(topic_messages) = messages_by_topic_locked.get_mut(&topic) {
                    self.send_unreceived_messages(client, &topic, topic_messages)?;
                }
            } else {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Error: no se pudo tomar lock a messages_by_topic para enviar Publish durante reconexión."));
            }
        }

        Ok(())
    }

    /// Analiza si el hashmap de PublishMessages del topic recibido por parámetro contiene o no mensajes que el user 'user' no haya
    /// recibido. Si sí los contiene, entonces se los envía, actualizando el last_id del 'user' para ese 'topic'.
    fn send_unreceived_messages(
        &self,
        user: &mut User,
        topic: &String,
        topic_messages: &VecDeque<PublishMessage>,
    ) -> Result<(), Error> {
        let diff = check_subscription_and_calculate_diff(user, topic, topic_messages)?;
        if diff < 0 {
            return Ok(()); // Si no está suscripto, no hay mensajes por enviar
        }
        send_unreceived_messages_to_user(user, topic, topic_messages, diff as u32)?;

        Ok(())
    }

    /// Agrega un usuario al hashmap de usuarios.
    pub fn add_new_user(
        &self,
        stream: &StreamType,
        username: &str,
        connect_msg: &ConnectMessage,
    ) -> Result<(), Error> {
        // Obtiene el will_message con todos sus campos relacionados necesarios, y lo guarda en el user,
        // para luego publicarlo al will_topic cuando user se desconecte
        let will_msg_info = connect_msg.get_will_to_publish();

        let username_c = username.to_string();
        //[] Aux: Nos guardamos el stream, volver a ver esto.
        let user = User::new(stream.try_clone()?, username_c.to_owned(), will_msg_info); //[]
        if let Ok(mut users) = self.connected_users.lock() {
            println!("Username agregado a la lista del server: {:?}", username);
            users.insert(username_c, user); //inserta el usuario en el hashmap
        }
        Ok(())
    }

    pub fn clone_ref(&self) -> Self {
        Self {
            connected_users: self.connected_users.clone(),
            available_packet_id: self.available_packet_id,
            messages_by_topic: self.messages_by_topic.clone(),
        }
    }

    /// Envía el will_message del user que se está desconectando, si tenía uno.
    pub fn publish_users_will_message(&self, username: &str) -> Result<(), Error> {
        let packet_id = 1000; // <-- aux: rever esto []: generate_packet_id requiere self mut, pero esto es multihilo, no tiene mucho sentido. Quizás un arc mutex u16, volver.
        let mut will_message_option = None;

        // Obtengo el will_message, si había uno.
        if let Ok(users) = self.connected_users.lock() {
            if let Some(user) = users.get(username) {
                will_message_option = user.get_publish_message_with(0, packet_id)?;
            }
        }

        // Suelto el lock, para que pueda tomarlo la función a la que estoy a punto de llamar.
        if let Some(will_message) = will_message_option {
            self.handle_publish_message(&will_message)?;
        }
        Ok(())
    }

    /// Procesa el PublishMessage: lo agrega al hashmap de su topic, y luego lo envía a los suscriptores de ese topic
    /// que estén conectados.
    pub fn handle_publish_message(&self, msg: &PublishMessage) -> Result<(), Error> {
        self.store_and_distribute_publish_msg(msg)?;
        self.remove_old_messages_from_server(msg.get_topic())?;
        Ok(())
    }

    /// Agrega los topics al suscriptor correspondiente. y devuelve los códigos de retorno(qos)
    pub fn add_topics_to_subscriber(
        &self,
        username: &str,
        msg: &SubscribeMessage,
    ) -> Result<Vec<SubscribeReturnCode>, Error> {
        let mut return_codes = vec![];

        // Agrega los topics a los que se suscribió el usuario
        if let Ok(mut connected_users) = self.connected_users.lock() {
            if let Some(user) = connected_users.get_mut(username) {
                for (topic, _qos) in msg.get_topic_filters() {
                    user.add_topic(topic.to_string());
                    return_codes.push(SubscribeReturnCode::QoS1);
                    println!(
                        "   Se agregó el topic {:?} al suscriptor {:?}",
                        topic, username
                    );
                }
            }
        }
        Ok(return_codes)
    }

    /// Envía un mensaje de tipo SubAck al cliente.
    pub fn send_suback_to(
        &self,
        client_id: &str,
        return_codes_res: &Result<Vec<SubscribeReturnCode>, Error>,
    ) -> Result<(), Error> {
        match return_codes_res {
            Ok(return_codes) => {
                let ack = SubAckMessage::new(0, return_codes.clone());
                let ack_msg_bytes = ack.to_bytes();
                if let Ok(mut connected_users_locked) = self.get_connected_users().lock() {
                    if let Some(user) = connected_users_locked.get_mut(client_id) {
                        user.write_message(&ack_msg_bytes)?;
                    }
                }
                println!("   tipo subscribe: Enviando el ack: {:?}", ack);
            }
            Err(e) => {
                println!("   ERROR: {:?}", e);
            }
        }
        Ok(())
    }

    /// Almacena el `PublishMessage` en la estructura del server para su topic, y lo envía a sus suscriptores.
    fn store_and_distribute_publish_msg(&self, msg: &PublishMessage) -> Result<(), Error> {
        // Vamos a recorrer todos los usuarios
        if let Ok(mut connected_users) = self.connected_users.lock() {
            // Necesitamos también los mensajes
            if let Ok(mut messages_by_topic_locked) = self.messages_by_topic.lock() {
                // Procesamos el mensaje
                self.add_message_to_topic_messages(msg.clone(), &mut messages_by_topic_locked);
                if let Some(topic_messages) = messages_by_topic_locked.get_mut(&msg.get_topic()) {
                    self.send_msgs_to_subscribers(
                        msg.get_topic(),
                        topic_messages,
                        &mut connected_users.values_mut(),
                    )?;
                }

            // Se devuelve error en los demás casos.
            } else {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Error: no se pudo tomar lock a messages_by_topic para almacenar y distribuir un Publish."));
            }
        } else {
            return Err(Error::new(
                ErrorKind::Other,
                "Error: no se pudo tomar lock a users para almacenar y distribuir un Publish.",
            ));
        }
        Ok(())
    }

    /// Devuelve si la estructura del topic contiene `PublishMessage`s.
    fn there_are_old_messages_to_send_for(
        &self,
        topic_messages: &VecDeque<PublishMessage>,
    ) -> bool {
        if !topic_messages.is_empty() {
            return true;
        }
        false
    }

    /// Devuelve si corresponde ejecutar la eliminación de mensajes anteriores de la estructura `topic_messages`.
    fn check_capacity(&self, topic_messages: &VecDeque<PublishMessage>) -> bool {
        if topic_messages.len() > TOPIC_MESSAGES_LEN {
            return true;
        }
        false
    }

    /// Envía a todos los suscriptores del topic `topic`, los mensajes que todavía no hayan recibido.
    fn send_msgs_to_subscribers(
        &self,
        topic: String,
        topic_messages: &VecDeque<PublishMessage>,
        users: &mut ValuesMut<'_, String, User>,
    ) -> Result<(), Error> {
        // Recorremos todos los usuarios
        for user in users {
            self.send_unreceived_messages(user, &topic, topic_messages)?;
        }
        Ok(())
    }

    // Remueve los mensajes antiguos de la estructuras de mensajes del topic `topic`, si la misma se encuentra cercana a una cierta capacidad fija.
    /// Para ello analiza primero el mínimo mensaje hasta el cual todos los usuarios conectados ya recibieron (el user `last_id``),
    /// borra hasta dicho mínimo, y luego actualiza la información de cada user (el user `last_id`) para que los índices sigan siendo consistentes.
    fn remove_old_messages_from_server(&self, topic: String) -> Result<(), Error> {
        // Vamos a recorrer los usuarios
        if let Ok(mut users_locked) = self.connected_users.lock() {
            let mut users = users_locked.values_mut();
            // Necesitamos también los mensajes
            if let Ok(mut messages_by_topic_locked) = self.messages_by_topic.lock() {
                if let Some(topic_messages) = messages_by_topic_locked.get_mut(&topic) {
                    if self.check_capacity(topic_messages) {
                        let min_last_id =
                            self.calculate_min_last_id_among_users_for(&topic, &mut users)?;

                        self.remove_messages_until(min_last_id, topic_messages)?;

                        let mut users = users_locked.values_mut(); // aux []

                        self.update_last_ids_for_users(&topic, min_last_id, &mut users)?;
                    }
                }
            } else {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Error: no se pudo tomar lock a messages_by_topic para remover elementos de la estructura para un topic."));
            }
        } else {
            return Err(Error::new(
                ErrorKind::Other,
                "Error: no se pudo tomar lock a users para recortar estructura de mensajes.",
            ));
        }
        Ok(())
    }

    /// Elimina todos los mensajes de la queue `topic_messages` que contiene los `PublishMessage`s deñ topic en cuestión,
    /// desde el principio hasta el `min_last_id` sin incluirlo.
    fn remove_messages_until(
        &self,
        min_last_id: u32,
        topic_messages: &mut VecDeque<PublishMessage>,
    ) -> Result<(), Error> {
        let mut i = 0;
        while i < min_last_id {
            topic_messages.pop_front();
            i += 1;
        }
        Ok(())
    }

    /// Actualiza, ajusta el last_id para el topic `topic`, de todos sus User's suscriptores, para mantenerlos consistentes luego de
    /// que se eliminen mensajes de la queue del topic `topic`.
    fn update_last_ids_for_users(
        &self,
        topic: &String,
        min_last_id: u32,
        users: &mut ValuesMut<'_, String, User>,
    ) -> Result<(), Error> {
        // Para cada user
        for user in users {
            // Si está suscripto al topic en cuestión
            if user.get_topics().contains(topic) {
                let last_id = user.get_last_id_by_topic(topic);
                let diff = last_id - min_last_id; // [] debug: esta resta también da panic.
                user.update_last_id_by_topic(topic, diff);
            }
        }

        Ok(())
    }

    /// Recorre los users para devolver, para el topic `topic`, el mínimo last_id de entre los users suscriptos a él.
    /// Devuelve el mínimo, o un error si no se pudo.
    fn calculate_min_last_id_among_users_for(
        &self,
        topic: &String,
        users: &mut ValuesMut<'_, String, User>,
    ) -> Result<u32, Error> {
        let mut min_last_id = u32::MAX;
        if users.len() == 0 {
            return Err(Error::new(
                ErrorKind::Other,
                "Error grave: calculate_min_last_id_among_users_for, se está por calcular el mínimo con error, lista de users vacía."));
        }

        // Recorro los usuarios
        for user in users {
            // Si el usuario está suscripto al topic
            if user.get_topics().contains(topic) {
                let user_last_id = user.get_last_id_by_topic(topic);
                // Tomamos el mínimo de los last_id de los usuarios suscriptos al topic
                if user_last_id < min_last_id {
                    min_last_id = user_last_id;
                }
            }
        }
        // Aux: entre todos ellos, algún mínimo sí o sí hay. Si todos tienen 0, el mínimo será 0.
        Ok(min_last_id)
    }

    /// Remueve al usuario `username` del hashmap de usuarios
    pub fn remove_user(&self, username: &str) {
        if let Ok(mut users) = self.connected_users.lock() {
            users.remove(username);
            println!("Username removido de la lista del server: {:?}", username);
            // debug
        }
    }

    /// Cambia el estado del usuario del server con username `username` a TemporallyDisconnected,
    /// para que no se le envíen mensajes si se encuentra en dicho estado y de esa forma evitar errores en writes.
    pub fn set_user_as_temporally_disconnected(&self, username: &str) -> Result<(), Error> {
        if let Ok(mut users) = self.connected_users.lock() {
            if let Some(user) = users.get_mut(username) {
                user.set_state(UserState::TemporallyDisconnected);
                println!(
                    "Username seteado como temporalmente desconectado: {:?}",
                    username
                );
            }
        }
        Ok(())
    }

    // Aux: esta función está comentada solo temporalmente mientras probamos algo, dsp se volverá a usar [].
    /// Envía un mensaje de tipo PubAck al cliente.
    pub fn send_puback_to(&self, client_id: &str, msg: &PublishMessage) -> Result<(), Error> {
        let option_packet_id = msg.get_packet_identifier();
        let packet_id = option_packet_id.unwrap_or(0);

        let ack = PubAckMessage::new(packet_id, 0);
        let ack_msg_bytes = ack.to_bytes();
        if let Ok(mut connected_users_locked) = self.get_connected_users().lock() {
            if let Some(user) = connected_users_locked.get_mut(client_id) {
                user.write_message(&ack_msg_bytes)?;
            }
        }
        println!(
            "   tipo publish: Enviado el ack para packet_id: {:?}",
            ack.get_packet_id()
        );
        Ok(())
    }

    /// Recorre la estructura de mensajes para el topic al que el suscriptor `username` se está suscribiendo con el `msg`,
    /// y le envía todos los mensajes que se publicaron a dicho topic previo a la suscripción.
    pub fn send_preexisting_msgs_to_new_subscriber(
        &self,
        username: &str,
        msg: &SubscribeMessage,
    ) -> Result<(), Error> {
        // Obtiene el topic al que se está suscribiendo el user
        for (topic, _) in msg.get_topic_filters() {
            // Al user que se conecta, se le envía lo que no tenía del topic en cuestión
            if let Ok(mut connected_users_locked) = self.connected_users.lock() {
                if let Some(user) = connected_users_locked.get_mut(username) {
                    // Necesitamos también los mensajes
                    if let Ok(mut messages_by_topic_locked) = self.messages_by_topic.lock() {
                        if let Some(topic_messages) = messages_by_topic_locked.get_mut(topic) {
                            if self.there_are_old_messages_to_send_for(topic_messages) {
                                self.send_unreceived_messages(user, topic, topic_messages)?;
                            }
                        }
                    } else {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Error: no se pudo tomar lock a messages_by_topic para enviar Publish durante un Subscribe."));
                    }
                }
            } else {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Error: no se pudo tomar lock a users para enviar Publish durante un Subscribe."));
            }
        }
        Ok(())
    }

    pub fn get_connected_users(&self) -> ShareableUsers {
        self.connected_users.clone()
    }
}

/// Crea un servidor en la dirección ip y puerto especificados.
fn create_server(ip: String, port: u16) -> Result<TcpListener, Error> {
    let listener =
        TcpListener::bind(format!("{}:{}", ip, port)).expect("Error al enlazar el puerto");
    Ok(listener)
}

// Verifica la suscripción y calcula la diferencia de mensajes
fn check_subscription_and_calculate_diff(
    user: &User,
    topic: &String,
    topic_messages: &VecDeque<PublishMessage>,
) -> Result<i32, Error> {
    let user_subscribed_topics = user.get_topics();
    if user_subscribed_topics.contains(topic) {
        let topic_server_last_id = topic_messages.len() as u32;
        let user_last_id = user.get_last_id_by_topic(topic);

        if user_last_id > topic_server_last_id {
            return Err(Error::new(
                ErrorKind::Other,
                "Error grave: send_unreceived_messages, la resta estaba por dar negativa.",
            ));
        }

        Ok((topic_server_last_id - user_last_id) as i32)
    } else {
        Ok(-1) // Si no está suscrito, no hay mensajes por enviar
    }
}

// Envia los mensajes no recibidos al usuario
fn send_unreceived_messages_to_user(
    user: &mut User,
    topic: &String,
    topic_messages: &VecDeque<PublishMessage>,
    diff: u32,
) -> Result<(), Error> {
    for _ in 0..diff {
        let next_message_index = user.get_last_id_by_topic(topic) as usize;
        if let Some(msg) = topic_messages.get(next_message_index) {
            user.write_message(&msg.to_bytes())?;
            user.update_last_id_by_topic(topic, (next_message_index + 1) as u32);
        } else {
            println!("ERROR NO SE ENCUENTRA EL TOPIC_MSGS.GET(TOPIC) A ENVIAR!!!");
        }
    }
    Ok(())
}
