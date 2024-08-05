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
    display_debug_puback_msg, display_debug_publish_msg, get_fixed_header_from_stream, get_fixed_header_from_stream_for_conn, get_whole_message_in_bytes_from_stream, is_disconnect_msg, shutdown, write_message_to_stream
};
use crate::mqtt::server::user_state::UserState;
use crate::mqtt::server::{file_helper::read_lines, user::User};

use std::collections::hash_map::ValuesMut;
use std::collections::{HashMap, VecDeque};
use std::fs::File;

use std::io::{Error, ErrorKind, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
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

        let mqtt_server_child = mqtt_server.clone_ref();

        let incoming_thread = std::thread::spawn(move || {
            if let Err(result) = mqtt_server_child.handle_incoming_connections(listener) {
                println!("Error al manejar las conexiones entrantes: {:?}", result);
            }
        });

        let _mqtt_server_another_child = mqtt_server.clone_ref();

        incoming_thread
            .join()
            .expect("Failed to join incoming thread");

        Ok(mqtt_server)
    }

    /// Agrega un usuario al hashmap de usuarios.
    fn add_new_user(
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

    /// Remueve al usuario `username` del hashmap de usuarios
    fn remove_user(&self, username: &str) {
        if let Ok(mut users) = self.connected_users.lock() {
            users.remove(username);
            println!("Username removido de la lista del server: {:?}", username);
            // debug
        }
    }

    /// Cambia el estado del usuario del server con username `username` a TemporallyDisconnected,
    /// para que no se le envíen mensajes si se encuentra en dicho estado y de esa forma evitar errores en writes.
    fn set_user_as_temporally_disconnected(&self, username: &str) -> Result<(), Error> {
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

    /// Envía el will_message del user que se está desconectando, si tenía uno.
    fn publish_users_will_message(&self, username: &str) -> Result<(), Error> {
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

    /// Busca al client_id en el hashmap de conectados, si ya existía analiza su estado:
    /// si ya estaba como activo, es un usuario duplicado por lo que le envía disconnect al stream anterior;
    /// si estaba como desconectado temporalmente (ie ctrl+C), se está reconectando.
    /// Devuelve true si era reconexión, false si no era reconexión.
    fn manage_possible_reconnecting_or_duplicate_user(
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
        write_message_to_stream(&msg.to_bytes(), &mut stream)?;
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

    /// Procesa el mensaje de conexión recibido, autentica
    /// al cliente y envía un mensaje de conexión de vuelta.
    fn process_connect(
        &self,
        fixed_header: &FixedHeader,
        mut stream: StreamType,
        fixed_header_buf: &[u8; 2],
    ) -> Result<(), Error> {
        // Continúa leyendo y reconstruye el mensaje recibido completo
        println!("Recibo mensaje tipo Connect");
        let msg_bytes =
            get_whole_message_in_bytes_from_stream(fixed_header, &mut stream, fixed_header_buf)?;
        let connect_msg = ConnectMessage::from_bytes(&msg_bytes);
        println!("Mensaje connect completo recibido: \n   {:?}", connect_msg);

        // Procesa el mensaje connect
        let (is_authentic, connack_response) =
            self.was_the_session_created_succesfully(&connect_msg)?;

        write_message_to_stream(&connack_response.to_bytes(), &mut stream)?;

        // Si el cliente se autenticó correctamente y se conecta por primera vez,
        // se agrega a la lista de usuarios conectados y se maneja la conexión
        if is_authentic {
            if let Some(username) = connect_msg.get_client_id() {
                // Busca en el hashmap, contempla casos si ya existía ese cliente:
                // lo desconecta si es duplicado, le envía lo que no tiene si se reconecta.
                let mut is_reconnection = false;
                if let Some(client_id) = connect_msg.get_client_id() {
                    is_reconnection =
                        self.manage_possible_reconnecting_or_duplicate_user(client_id, &stream)?;
                }

                // Si es reconexión, no quiero add_new_user xq eso me crearía un User nuevo y yo ya tengo uno.
                if !is_reconnection {
                    self.add_new_user(&stream, username, &connect_msg)?;
                }

                // Continuar la conexión, ya puede empezar a recibir otros tipos de mensaje (publish msg / subscribe msg / etc).
                self.handle_connection(username, &mut stream)?;
                println!(
                    "   se ha salido de la función, se deja de leer para el cliente: {}.",
                    username
                ); // debug
            }
        } else {
            println!("   ERROR: No se pudo autenticar al cliente."); //(ya se le envió el ack informando).
        }

        Ok(())
    }

    fn is_guest_mode_active(&self, user: Option<&String>, passwd: Option<&String>) -> bool {
        user.is_none() && passwd.is_none()
    }

    /// Autentica al usuario con las credenciales almacenadas en el archivo credentials.txt
    fn authenticate(&self, user: Option<&String>, passwd: Option<&String>) -> bool {
        let mut is_authentic: bool = false;
        let credentials_path = Path::new("credentials.txt");
        if let Ok(lines) = read_lines(credentials_path) {
            for line in lines.map_while(Result::ok) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() != 2 {
                    continue;
                }
                let username = parts[0]; // username
                let password = parts[1]; // password
                if let Some(msg_user) = user {
                    if let Some(msg_passwd) = passwd {
                        is_authentic = msg_user == username && msg_passwd == password;
                        if is_authentic {
                            break;
                        }
                    }
                }
            }
        }
        is_authentic
    }

    /// Verifica si la sesión fue creada exitosamente: usuario valido o invitado
    /// y devuelve un mensaje CONNACK acorde.
    fn was_the_session_created_succesfully(
        &self,
        connect_msg: &ConnectMessage,
    ) -> Result<(bool, ConnackMessage), Error> {
        if self.is_guest_mode_active(connect_msg.get_user(), connect_msg.get_passwd())
            || self.authenticate(connect_msg.get_user(), connect_msg.get_passwd())
        {
            // Aux: volver cuando haya tema desconexiones, acá le estoy pasando siempre un SessionPresent::NotPresentInLastSession. [].
            let connack_response = ConnackMessage::new(
                SessionPresent::NotPresentInLastSession,
                ConnectReturnCode::ConnectionAccepted,
            );
            Ok((true, connack_response))
        } else {
            let connack_response = ConnackMessage::new(
                SessionPresent::NotPresentInLastSession,
                ConnectReturnCode::NotAuthorized,
            );
            Ok((false, connack_response))
        }
    }

    /// Maneja la conexión con el cliente, recibe mensajes y los procesa.
    fn handle_connection(&self, username: &str, stream: &mut StreamType) -> Result<(), Error> {
        let mut fixed_header_info: ([u8; 2], FixedHeader);
        println!("Mqtt cliente leyendo: esperando más mensajes.");

        loop {
            match get_fixed_header_from_stream(stream) {
                Ok(Some((fixed_h_buf, fixed_h))) => {
                    fixed_header_info = (fixed_h_buf, fixed_h);

                    // Caso se recibe un disconnect
                    if is_disconnect_msg(&fixed_header_info.1) {
                        self.publish_users_will_message(username)?;
                        self.remove_user(username);
                        println!("Mqtt cliente leyendo: recibo disconnect");
                        shutdown(stream);
                        break;
                    }

                    self.process_message(username, stream, &fixed_header_info)?;
                    // esta función lee UN mensaje.
                }
                Ok(None) => {
                    println!("Se desconectó el cliente: {:?}.", username);
                    self.set_user_as_temporally_disconnected(username)?;
                    self.publish_users_will_message(username)?;
                    // Acá se manejaría para recuperar la sesión cuando se reconecte.
                    break;
                }
                Err(_) => todo!(),
            }
        }
        Ok(())
    }

    // Aux: esta función está comentada solo temporalmente mientras probamos algo, dsp se volverá a usar [].
    /// Envía un mensaje de tipo PubAck al cliente.
    fn send_puback(&self, msg: &PublishMessage, stream: &mut StreamType) -> Result<(), Error> {
        let option_packet_id = msg.get_packet_identifier();
        let packet_id = option_packet_id.unwrap_or(0);

        let ack = PubAckMessage::new(packet_id, 0);
        let ack_msg_bytes = ack.to_bytes();
        write_message_to_stream(&ack_msg_bytes, stream)?;
        println!("   tipo publish: Enviado el ack para packet_id: {:?}", ack.get_packet_id());
        Ok(())
    }

    /// Agrega los topics al suscriptor correspondiente. y devuelve los códigos de retorno(qos)
    fn add_topics_to_subscriber(
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
    fn send_suback(
        &self,
        return_codes: Vec<SubscribeReturnCode>,
        stream: &mut StreamType,
    ) -> Result<(), Error> {
        let ack = SubAckMessage::new(0, return_codes);
        let ack_msg_bytes = ack.to_bytes();

        write_message_to_stream(&ack_msg_bytes, stream)?;
        println!("   tipo subscribe: Enviando el ack: {:?}", ack);

        Ok(())
    }

    /// Se ejecuta una vez recibido un `ConnectMessage` exitoso y devuelto un `ConnAckMessage` acorde.
    /// Se puede empezar a recibir mensajes de otros tipos (`Publish`, `Subscribe`), de este cliente.
    /// Recibe el `stream` para responder acks al cliente en cuestión.
    /// Procesa Un mensaje, acorde a su tipo.
    fn process_message(
        &self,
        username: &str,
        stream: &mut StreamType,
        fixed_header_info: &([u8; 2], FixedHeader),
    ) -> Result<(), Error> {
        let (fixed_header_bytes, fixed_header) = fixed_header_info;

        // Lee la segunda parte del mensaje y junto ambas partes (concatena con el fixed_header)
        let msg_bytes =
            get_whole_message_in_bytes_from_stream(fixed_header, stream, fixed_header_bytes)?;

        // Ahora sí ya puede haber diferentes tipos de mensaje.
        match fixed_header.get_message_type() {
            PacketType::Publish => {
                let msg = PublishMessage::from_bytes(msg_bytes)?;
                //println!("   Mensaje publish completo recibido: {:?}", msg);
                display_debug_publish_msg(&msg);
                self.send_puback(&msg, stream)?;
                self.handle_publish_message(&msg)?;
            }
            PacketType::Subscribe => {
                let msg = SubscribeMessage::from_bytes(msg_bytes)?;
                let return_codes = self.add_topics_to_subscriber(username, &msg)?;

                self.send_preexisting_msgs_to_new_subscriber(username, &msg)?;

                self.send_suback(return_codes, stream)?;
                println!("DEBUG: terminado el subscribe, para user: {:?}", username);
            }
            PacketType::Puback => {
                let msg = PubAckMessage::msg_from_bytes(msg_bytes)?;
                display_debug_puback_msg(&msg);
            }
            _ => println!(
                "   ERROR: tipo desconocido: recibido: \n   {:?}",
                fixed_header
            ),
        };

        Ok(())
    }

    /// Recorre la estructura de mensajes para el topic al que el suscriptor `username` se está suscribiendo con el `msg`,
    /// y le envía todos los mensajes que se publicaron a dicho topic previo a la suscripción.
    fn send_preexisting_msgs_to_new_subscriber(&self, username: &str, msg: &SubscribeMessage) -> Result<(), Error> {
        // Obtiene el topic al que se está suscribiendo el user
        for (topic, _) in msg.get_topic_filters() {
            // Al user que se conecta, se le envía lo que no tenía del topic en cuestión
            if let Ok(mut connected_users_locked) = self.connected_users.lock() {
                if let Some(user) = connected_users_locked.get_mut(username) {

                    // Necesitamos también los mensajes
                    if let Ok(mut messages_by_topic_locked) = self.messages_by_topic.lock() {
                        if let Some(topic_messages) = messages_by_topic_locked.get_mut(topic) {
                            if self.there_are_old_messages_to_send_for(&topic_messages) {
                                self.send_unreceived_messages(user, topic, topic_messages)?;
                            }
                        }
                    } else {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Error: no se pudo tomar lock a messages_by_topic para enviar Publish durante un Subscribe."));
                    }
                }
            }  else {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Error: no se pudo tomar lock a users para enviar Publish durante un Subscribe."));
            }                    
        }
        Ok(())
    }

    /// Procesa los mensajes entrantes de un dado cliente.
    fn handle_client(&self, mut stream: StreamType) -> Result<(), Error> {
        println!("Mqtt cliente leyendo: esperando más mensajes.");

        // Leo un fixed header, deberá ser de un connect
        let (fixed_header_buf, fixed_header) = get_fixed_header_from_stream_for_conn(&mut stream)?;

        let fixed_header_info = (fixed_header_buf, fixed_header);
        let (fixed_header_buf, fixed_header) = (&fixed_header_info.0, &fixed_header_info.1);

        // El único tipo válido es el de connect, xq siempre se debe iniciar la comunicación con un connect.
        match fixed_header.get_message_type() {
            PacketType::Connect => {
                self.process_connect(fixed_header, stream, fixed_header_buf)?;
            }
            _ => {
                println!("Error, el primer mensaje recibido DEBE ser un connect.");
                println!("   recibido: {:?}", fixed_header);
                println!("Cerrando la conexión.");
                shutdown(&stream);
            }
        };
        Ok(())
    }

    fn clone_ref(&self) -> Self {
        Self {
            connected_users: self.connected_users.clone(),
            available_packet_id: self.available_packet_id,
            messages_by_topic: self.messages_by_topic.clone(),
        }
    }

    /// Maneja las conexiones entrantes, acepta las conexiones y crea un hilo para manejar cada una.
    fn handle_incoming_connections(&self, listener: TcpListener) -> Result<(), Error> {
        println!("Servidor iniciado. Esperando conexiones.");
        let mut handles = vec![];

        for stream_client in listener.incoming() {
            match stream_client {
                Ok(stream_client) => {
                    let self_child = self.clone_ref();
                    let handle = std::thread::spawn(move || {
                        let _ = self_child.handle_client(stream_client);
                    });
                    handles.push(handle);
                }
                Err(e) => {
                    println!("Error al aceptar la conexión: {}", e);
                }
            }
        }

        // Espera a que todos los hilos terminen.
        for handle in handles {
            if let Err(e) = handle.join() {
                eprintln!("Error al esperar el hilo: {:?}", e);
            }
        }

        Ok(())
    }

    /// Agrega un PublishMessage a la estructura de mensajes de su topic.
    fn add_message_to_topic_messages(&self, publish_msg: PublishMessage, msgs_by_topic_l: &mut std::sync::MutexGuard<'_, HashMap<String, TopicMessages>>) {
        let topic = publish_msg.get_topic();
    
        // Obtiene o crea (si no existía) el VeqDequeue<PublishMessage> correspondiente al topic del publish message
        let topic_messages = msgs_by_topic_l
            .entry(topic)
            //.or_insert_with(VecDeque::new);
            .or_default(); // clippy.
        // Inserta el PublishMessage en el HashMap interno
        topic_messages.push_back(publish_msg);        
    }

    /// Procesa el PublishMessage: lo agrega al hashmap de su topic, y luego lo envía a los suscriptores de ese topic
    /// que estén conectados.
    fn handle_publish_message(&self, msg: &PublishMessage) -> Result<(), Error> {
        self.store_and_distribute_publish_msg(msg)?;
        self.remove_old_messages_from_server(msg.get_topic())?;
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
                    self.send_msgs_to_subscribers(msg.get_topic(), topic_messages, &mut connected_users.values_mut())?;
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
                "Error: no se pudo tomar lock a users para almacenar y distribuir un Publish."));
        }
        Ok(())
    }

    /// Envía a todos los suscriptores del topic `topic`, los mensajes que todavía no hayan recibido.
    fn send_msgs_to_subscribers(&self, topic: String, topic_messages: &VecDeque<PublishMessage>, users: &mut ValuesMut<'_, String, User>) -> Result<(), Error> {
        // Recorremos todos los usuarios        
        for user in users {
            self.send_unreceived_messages(user, &topic, topic_messages)?;
        }
        Ok(())
    }

    // (0,1,2,3,4,5) len = 6
    // server: 5 last server
    // user: 3 este es mi last

    /// Analiza si el hashmap de PublishMessages del topic recibido por parámetro contiene o no mensajes que el user 'user' no haya
    /// recibido. Si sí los contiene, entonces se los envía, actualizando el last_id del 'user' para ese 'topic'.
    fn send_unreceived_messages(&self, user: &mut User, topic: &String, topic_messages: &VecDeque<PublishMessage>) -> Result<(), Error> {
        // Si user está suscripto al topic en cuestión
        let user_subscribed_topics = user.get_topics();
        if user_subscribed_topics.contains(topic) {
            // Calculamos la cantidad de mensajes de topic que a user le falta recibir
            let topic_server_last_id = topic_messages.len() as u32;
            let user_last_id = user.get_last_id_by_topic(topic);
            
            if user_last_id > topic_server_last_id {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Error grave: send_unreceived_messages, la resta estaba por dar negativa."));
            }

            let diff = topic_server_last_id - user_last_id; // Debug: Panic! Xq overflow, da negativo. Hola lock envenenado.
            println!(
                "DEBUG: last server: {}, last user: {}, diff: {}, user: {:?}",
                topic_server_last_id, user_last_id, diff, user.get_username()
            );
            for _ in 0..diff {
                // de 0 a diff, sin incluir el diff, "[0, diff)";
                let next_message = user.get_last_id_by_topic(topic) as usize;                        
                if let Some(msg) = topic_messages.get(next_message){
                    
                    let mut user_stream = user.get_stream()?;
                    if user.is_not_disconnected() {
                        write_message_to_stream(&msg.to_bytes(), &mut user_stream)?;
                        println!("[DEBUG]:   el msg se envió.");
                        user.update_last_id_by_topic(topic, (next_message + 1) as u32);
                    } else {
                        // rama else solamente para debug
                        println!("[DEBUG]:   el msg No se envía xq entra al if.");
                    }
                } else {
                    println!("ERROR NO SE ENCUENTRA EL TOPIC_MSGS.GET(TOPIC) A ENVIAR!!!");
                }
                
            }
        }
        Ok(())
    }

    /// Devuelve si la estructura del topic contiene `PublishMessage`s.
    fn there_are_old_messages_to_send_for(&self, topic_messages: &VecDeque<PublishMessage>) -> bool {        
        if !topic_messages.is_empty() {
            return true;
        }
        false
    }

    /// Devuelve si corresponde ejecutar la eliminación de mensajes anteriores de la estructura `topic_messages`.
    fn check_capacity(&self, topic_messages: &VecDeque<PublishMessage>) -> bool {        
        if topic_messages.len() > 10 {
            return true;
        }
        false
    }
    
    /// Remueve los mensajes antiguos de la estructuras de mensajes del topic `topic`, si la misma se encuentra cercana a una cierta capacidad fija.
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
                
                        let min_last_id = self.calculate_min_last_id_among_users_for(&topic, &mut users)?;

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
                "Error: no se pudo tomar lock a users para recortar estructura de mensajes."))
        }
        Ok(())
    }
    
    /// Recorre los users para devolver, para el topic `topic`, el mínimo last_id de entre los users suscriptos a él.
    /// Devuelve el mínimo, o un error si no se pudo.
    fn calculate_min_last_id_among_users_for(&self, topic: &String, users: &mut ValuesMut<'_, String, User>) -> Result<u32, Error> {
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
                println!( "DEBUG: last_id: {:?} de user: {:?} ", user_last_id, user.get_username());
                
            }
        }
        // Aux: entre todos ellos, algún mínimo sí o sí hay. Si todos tienen 0, el mínimo será 0.
        Ok(min_last_id)
        
    }
    
    /// Actualiza, ajusta el last_id para el topic `topic`, de todos sus User's suscriptores, para mantenerlos consistentes luego de
    /// que se eliminen mensajes de la queue del topic `topic`.
    fn update_last_ids_for_users(&self, topic: &String, min_last_id: u32, users: &mut ValuesMut<'_, String, User>) -> Result<(), Error> {
        println!("DEBUG: Entrando a update_last_ids_for_users, min_last_id calculado fue: {}", min_last_id);
        // Para cada user
        for user in users {               
            println!("DEBUG:   user: {:?} tiene get_topics: {:?}", user.get_username(), user.get_topics());
            // Si está suscripto al topic en cuestión
            if user.get_topics().contains(topic) {
                let last_id = user.get_last_id_by_topic(topic);
                let diff = last_id - min_last_id; // [] debug: esta resta también da panic.
                user.update_last_id_by_topic(topic, diff);
                println!("DEBUG: last_id anterior: {}, queda: {}, para user: {:?}.", last_id, diff, user.get_username());
            }
        }
        println!("DEBUG: Saliendo de update_last_ids_for_users."); // debug
        
        Ok(())
    }

    /// Elimina todos los mensajes de la queue `topic_messages` que contiene los `PublishMessage`s deñ topic en cuestión,
    /// desde el principio hasta el `min_last_id` sin incluirlo.
    fn remove_messages_until(&self, min_last_id: u32, topic_messages: &mut VecDeque<PublishMessage>) -> Result<(), Error> {
        println!("DEBUG: remove_messages_until, cola de mensajes antes del remove: {:?}", topic_messages);
        let mut i = 0;
        while i < min_last_id { // [] sah, es un for
            topic_messages.pop_front();
            i += 1;
        }
        println!("DEBUG: remove_messages_until, cola de mensajes después del remove: {:?}", topic_messages);
    
        Ok(())
    }    

}

/// Crea un servidor en la dirección ip y puerto especificados.
fn create_server(ip: String, port: u16) -> Result<TcpListener, Error> {
    let listener =
        TcpListener::bind(format!("{}:{}", ip, port)).expect("Error al enlazar el puerto");
    Ok(listener)
}
