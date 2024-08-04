use crate::mqtt::messages::packet_type::PacketType;
use crate::mqtt::messages::{
    connack_message::ConnackMessage, connack_session_present::SessionPresent,
    connect_message::ConnectMessage, connect_return_code::ConnectReturnCode,
    disconnect_message::DisconnectMessage, puback_message::PubAckMessage,
    publish_message::PublishMessage, suback_message::SubAckMessage,
    subscribe_message::SubscribeMessage, subscribe_return_code::SubscribeReturnCode,
};
// Add the missing import
use crate::mqtt::mqtt_utils::fixed_header::FixedHeader;
use crate::mqtt::mqtt_utils::utils::{
    display_debug_puback_msg, display_debug_publish_msg, get_fixed_header_from_stream,
    get_fixed_header_from_stream_for_conn, get_whole_message_in_bytes_from_stream,
    is_disconnect_msg, shutdown, write_message_to_stream,
};
use crate::mqtt::server::{file_helper::read_lines, user::User};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

use crate::mqtt::server::user_state::UserState;
use std::collections::hash_map::ValuesMut;
use std::collections::{HashMap, VecDeque};
use std::fs::File;

use std::io::{Error, ErrorKind, Write};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

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
    tx_2: Option<Sender<Vec<u8>>>,
}

impl MQTTServer {
    pub fn new() -> Result<Self, Error> {
        let file_path = "log.txt";
        if let Err(e) = clean_file(file_path) {
            println!("Error al limpiar el archivo: {:?}", e);
        }

        let mqtt_server = Self {
            connected_users: Arc::new(Mutex::new(HashMap::new())),
            available_packet_id: 0,
            messages_by_topic: Arc::new(Mutex::new(HashMap::new())),
            tx_2: None,
        };

        Ok(mqtt_server)
    }

    /// Agrega un PublishMessage al Hashmap de su topic
    fn add_message_to_hashmap(
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
        let mut stream = client.get_stream()?;
        //write_message_to_stream(&msg.to_bytes(), &mut stream)?;
        self.write_to_client(msg.to_bytes());
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
    pub fn send_unreceived_messages(
        &self,
        user: &mut User,
        topic: &String,
        topic_messages: &VecDeque<PublishMessage>,
    ) -> Result<(), Error> {
        // Si user está suscripto al topic en cuestión
        let user_subscribed_topics = user.get_topics();
        if user_subscribed_topics.contains(topic) {
            // Calculamos la cantidad de mensajes de topic que a user le falta recibir
            let topic_server_last_id = topic_messages.len() as u32;
            let user_last_id = user.get_last_id_by_topic(topic);
            let diff = topic_server_last_id - user_last_id; // Debug: Panic! Xq overflow, da negativo. Hola lock envenenado.
            for _ in 0..diff {
                // de 0 a diff, sin incluir el diff, "[0, diff)";
                let next_message = user.get_last_id_by_topic(topic) as usize;
                if let Some(msg) = topic_messages.get(next_message) {
                    let mut user_stream = user.get_stream()?;
                    if user.is_not_disconnected() {
                        write_message_to_stream(&msg.to_bytes(), &mut user_stream)?;
                        println!("[DEBUG]:   el msg se envió.");
                        user.update_last_id_by_topic(topic, (next_message + 1) as u32);
                    } else {
                        // rama else solamente para debug
                        println!("[DEBUG]:   el msg NO se envía xq entra al if.");
                    }
                } else {
                    println!("ERROR NO SE ENCUENTRA EL TOPIC_MSGS.GET(TOPIC) A ENVIAR!!!");
                }
            }
        }
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

        //[] Aux: Nos guardamos el stream, volver a ver esto.
        let user = User::new(stream.try_clone()?, username.to_string(), will_msg_info); //[]
        if let Ok(mut users) = self.connected_users.lock() {
            let username = user.get_username();
            println!("Username agregado a la lista del server: {:?}", username);
            users.insert(username, user); //inserta el usuario en el hashmap
        }
        Ok(())
    }

    pub fn write_connack_to_stream(&self, connack: ConnackMessage, stream: &mut StreamType) {
        write_message_to_stream(&connack.to_bytes(), stream);
    }

    pub fn clone_ref(&self) -> Self {
        Self {
            connected_users: self.connected_users.clone(),
            available_packet_id: self.available_packet_id,
            messages_by_topic: self.messages_by_topic.clone(),
            tx_2: self.tx_2.clone(),
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
        //if self.check_capacity(msg.get_topic()) { // <--- aux: se mueve para adentro del remove_old_messages.

        match self.remove_old_messages(msg.get_topic()) {
            // print para ver el error mientras debuggueamos
            Ok(_) => {}
            Err(e) => println!("DEBUG: Error al salir del remove_old_messages: {:?}", e),
        };
        //}

        self.store_and_distribute_publish_msg(msg)?; //

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
    pub fn send_suback(
        &self,
        return_codes: Vec<SubscribeReturnCode>,
    ) -> Result<(), Error> {
        let ack = SubAckMessage::new(0, return_codes);
        let ack_msg_bytes = ack.to_bytes();

        //write_message_to_stream(&ack_msg_bytes, stream)?;
        self.write_to_client(ack_msg_bytes);
        println!("   tipo subscribe: Enviando el ack: {:?}", ack);

        Ok(())
    }

    /// Almacena el `PublishMessage` en la estructura del server para su topic, y lo envía a sus suscriptores.
    fn store_and_distribute_publish_msg(&self, msg: &PublishMessage) -> Result<(), Error> {
        println!("DEBUG: entrando a store_and_distribute_publish_msg, por tomar lock");
        if let Ok(mut messages_by_topic_locked) = self.messages_by_topic.lock() {
            self.add_message_to_hashmap(msg.clone(), &mut messages_by_topic_locked); // workaround, la ubico acá por ahora xq adentro del if ya es otro tipo de dato y cambiaría implementación.

            if let Some(topic_messages) = messages_by_topic_locked.get_mut(&msg.get_topic()) {
                // Sin soltar el lock, también necesitamos los users
                self.send_msgs_to_subscribers(msg.get_topic(), topic_messages)?;
            }
        } else {
            return Err(Error::new(
                ErrorKind::Other,
                "Error: no se pudo tomar lock a messages_by_topic para almacenar y distribuir un Publish."));
        }
        println!("DEBUG: saliendo de store_and_distribute_publish_msg");
        Ok(())
    }

    /// Devuelve si la estructura del topic contiene `PublishMessage`s.
    pub fn there_are_old_messages_to_send_for(&self, topic: &String) -> bool {
        if let Ok(messages_by_topic_locked) = self.messages_by_topic.lock() {
            if let Some(topic_messages) = messages_by_topic_locked.get(topic) {
                if !topic_messages.is_empty() {
                    return true;
                }
            }
        }
        false
    }

    /// Envía a todos los suscriptores del topic `topic`, los mensajes que todavía no hayan recibido.
    fn send_msgs_to_subscribers(
        &self,
        topic: String,
        topic_messages: &VecDeque<PublishMessage>,
    ) -> Result<(), Error> {
        // Recorremos todos los usuarios
        if let Ok(mut connected_users) = self.connected_users.lock() {
            for user in connected_users.values_mut() {
                self.send_unreceived_messages(user, &topic, topic_messages)?;
            }
        }

        Ok(())
    }

    /// .
    fn remove_old_messages(&self, topic: String) -> Result<(), Error> {
        println!("DEBUG: entrando a remove_old_messages, por tomar lock");
        // Recorro los usuarios
        if let Ok(mut users_locked) = self.connected_users.lock() {
            let mut users = users_locked.values_mut();
            // Necesitamos también los mensajes
            if let Ok(mut messages_by_topic_locked) = self.messages_by_topic.lock() {
                if let Some(topic_messages) = messages_by_topic_locked.get_mut(&topic) {
                    if topic_messages.len() > 10 {
                        let min_last_id =
                            self.calculate_min_last_id_among_users_for(&topic, &mut users)?;
                        //println!( "DEBUG: 1 _ min_last_id: {:?}", min_last_id);

                        self.remove_messages_until(min_last_id, topic_messages)?;
                        //println!( "DEBUG: 2 _ min_last_id: {:?}", min_last_id);

                        let mut users = users_locked.values_mut(); // aux []

                        self.update_last_ids_for_users(&topic, min_last_id, &mut users)?;
                        //println!( "DEBUG: 3 _ min_last_id: {:?}", min_last_id);
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
        println!("DEBUG: saliendo de remove_old_messages");
        /*
        // Aux: así estaba antes:
        let min_last_id = self.calculate_min_last_id_among_users_for(&topic)?;
        println!( "DEBUG: 1 _ min_last_id: {:?}", min_last_id);

        self.remove_messages_until(min_last_id, &topic)?;
        println!( "DEBUG: 2 _ min_last_id: {:?}", min_last_id);

        // self.update_last_ids_for_users(&topic, min_last_id)?;
        // println!( "DEBUG: 3 _ min_last_id: {:?}", min_last_id);
        */
        Ok(())
    }

    /// Elimina todos los mensajes de la queue `topic_messages` que contiene los `PublishMessage`s deñ topic en cuestión,
    /// desde el principio hasta el `min_last_id` sin incluirlo.
    fn remove_messages_until(
        &self,
        min_last_id: u32,
        topic_messages: &mut VecDeque<PublishMessage>,
    ) -> Result<(), Error> {
        println!("DEBUG: Entrando a remove_messages_until");

        println!(
            "DEBUG: Cola de mensajes antes del remove: {:?}",
            topic_messages
        );
        let mut i = 0;
        while i < min_last_id {
            // [] sah, es un for
            topic_messages.pop_front();
            i += 1;
        }

        println!(
            "DEBUG: Cola de mensajes después del remove: {:?}",
            topic_messages
        );
        println!("DEBUG: Saliendo de remove_messages_until");

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
        println!(
            "DEBUG: Entrando a update_last_ids_for_users, min_last_id calculado fue: {}",
            min_last_id
        );
        // Para cada user
        for user in users {
            println!(
                "DEBUG:   user: {:?} tiene get_topics: {:?}",
                user.get_username(),
                user.get_topics()
            );
            // Si está suscripto al topic en cuestión
            if user.get_topics().contains(topic) {
                let last_id = user.get_last_id_by_topic(topic);
                let diff = last_id - min_last_id; // [] debug: esta resta también da panic.
                user.update_last_id_by_topic(topic, diff);
                println!(
                    "DEBUG: last_id anterior: {}, queda: {}, para user: {:?}.",
                    last_id,
                    diff,
                    user.get_username()
                );
            }
        }
        println!("DEBUG: Saliendo de update_last_ids_for_users."); // debug

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
        // Recorro los usuarios
        for user in users {
            // Si el usuario está suscripto al topic
            if user.get_topics().contains(topic) {
                let user_last_id = user.get_last_id_by_topic(topic);
                // Tomamos el mínimo de los last_id de los usuarios suscriptos al topic
                if user_last_id < min_last_id {
                    min_last_id = user_last_id;
                }
                println!(
                    "DEBUG: LAST ID: {:?} de USER: {:?} ",
                    user_last_id,
                    user.get_username()
                );
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

    pub fn set_tx_sender(&mut self, tx: Sender<Vec<u8>>) {
        self.tx_2 = Some(tx);
    }

    pub fn write_to_client(&self, msg: Vec<u8>) {
        if let Some(tx) = &self.tx_2 {
            let send_res = tx.send(msg);
            match send_res {
                Ok(_) => println!("Mensaje enviado exitosamente al cliente."),
                Err(_) => println!("Error al enviar mensaje al cliente."),
            }
        }
    }

    // Aux: esta función está comentada solo temporalmente mientras probamos algo, dsp se volverá a usar [].
    /// Envía un mensaje de tipo PubAck al cliente.
    pub fn send_puback(&self, msg: &PublishMessage) -> Result<(), Error> {
        let option_packet_id = msg.get_packet_identifier();
        let packet_id = option_packet_id.unwrap_or(0);

        let ack = PubAckMessage::new(packet_id, 0);
        let ack_msg_bytes = ack.to_bytes();
        //write_message_to_stream(&ack_msg_bytes, stream)?;
        self.write_to_client(ack_msg_bytes);
        println!("   tipo publish: Enviado el ack para packet_id: {:?}", ack.get_packet_id());
        Ok(())
    }
}
