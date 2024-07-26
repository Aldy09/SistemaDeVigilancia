use crate::mqtt::messages::packet_type::PacketType;
use crate::mqtt::messages::{
    connack_message::ConnackMessage, connack_session_present::SessionPresent,
    connect_message::ConnectMessage, connect_return_code::ConnectReturnCode,
    disconnect_message::DisconnectMessage, puback_message::PubAckMessage,
    publish_message::PublishMessage, suback_message::SubAckMessage,
    subscribe_message::SubscribeMessage, subscribe_return_code::SubscribeReturnCode,
};
// Add the missing import
use crate::mqtt::mqtt_utils::utils::{
    get_fixed_header_from_stream, get_fixed_header_from_stream_for_conn,
    get_whole_message_in_bytes_from_stream, is_disconnect_msg, shutdown, write_message_to_stream,
};
use crate::mqtt::mqtt_utils::fixed_header::FixedHeader;
use crate::mqtt::server::user_state::UserState;
use crate::mqtt::server::{connected_user::User, file_helper::read_lines};

use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};

type ShareableUsers = Arc<Mutex<HashMap<String, User>>>;
type StreamType = TcpStream;

fn clean_file(file_path: &str) -> Result<(), Error> {
    let mut file = File::create(file_path)?;
    file.write_all(b"")?; // Escribe un contenido vacío para limpiarlo
    Ok(())
}

#[derive(Debug)]
pub struct MQTTServer {
    connected_users: ShareableUsers,
    available_packet_id: u16, //
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

    /// Agrega un usuario al hashmap de usuarios conectados
    fn add_user(&self, stream: &StreamType, username: &str, connect_msg: &ConnectMessage) -> Result<(), Error> {
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

    /// Remueve al usuario `username` del hashmap de usuarios conectados
    fn remove_user(&self, username: &str) {
        if let Ok(mut users) = self.connected_users.lock() {
            users.remove(username);
            println!("Username removido de la lista del server: {:?}", username);
            // debug
        }
    }

    /// Cambia el estado del usuario del server con username `username` a TemporallyDisconnected,
    /// para que no se le envíen mensajes si se encuentra en dicho estado y de esa forma evitar errores en writes.
    fn set_user_as_temporally_disconnected(&self, username: &str) -> Result<(), Error>{
        if let Ok(mut users) = self.connected_users.lock() {
            if let Some(user ) = users.get_mut(username){
                user.set_state(UserState::TemporallyDisconnected);
                println!("Username seteado como temporalmente desconectado: {:?}", username);    
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
            if let Some(user ) = users.get(username){
                will_message_option = user.get_publish_message_with(0, packet_id)?;
            }
        }

        // Suelto el lock, para que pueda tomarlo la función a la que estoy a punto de llamar.
        if let Some(will_message) = will_message_option{
            self.handle_publish_message(&will_message)?;
        }
        Ok(())
    }

    /// Devuelve el packet_id a usar para el siguiente mensaje enviado.
    /// Incrementa en 1 el atributo correspondiente, debido a la llamada anterior, y devuelve el valor a ser usado
    /// en el envío para el cual fue llamada esta función.
    fn generate_packet_id(&mut self) -> u16 {
        self.available_packet_id += 1;
        self.available_packet_id
    }

    /// Busca al client_id en el hashmap de conectados, si ya existía analiza su estado:
    /// si ya estaba como activo, es un usuario duplicado por lo que le envía disconnect al stream anterior;
    /// si estaba como desconectado temporalmente (ie ctrl+C), se está reconectando.
    fn manage_possible_reconnecting_or_duplicate_user(
        &self,
        client_id: &str,
    ) -> Result<(), Error> {
        if let Ok(mut connected_users_locked) = self.connected_users.lock() {
            //if let Some(client) = connected_users_locked.remove(client_id) { // <-- aux: volver [].
            if let Some(client) = connected_users_locked.get_mut(client_id) {
                match client.get_state() {
                    UserState::Active => {
                        // disconnect_previous_client_if_already_connected:
                        let msg = DisconnectMessage::new();
                        let mut stream = client.get_stream()?; //

                        write_message_to_stream(&msg.to_bytes(), &mut stream)?;
                    },
                    UserState::TemporallyDisconnected => {
                        // Vuelve a setearle estado activo, para que se le hagan los writes.
                        client.set_state(UserState::Active);
                        // Y acá iría la lógica para enviarle lo que pasó mientras no estuvo en línea.
                    },
                }
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

        // Busca en el hashmap, contempla casos si ya existía ese cliente:
        // lo desconecta si es duplicado, le envía lo que no tiene si se reconecta.
        if let Some(client_id) = connect_msg.get_client_id() {
            self.manage_possible_reconnecting_or_duplicate_user(client_id)?;
        }

        write_message_to_stream(&connack_response.to_bytes(), &mut stream)?;
        println!("   tipo connect: Enviado el ack: {:?}", connack_response);

        // Si el cliente se autenticó correctamente y se conecta por primera vez,
        // se agrega a la lista de usuarios conectados(add_user) y se maneja la conexión(handle_connection)
        if is_authentic {
            if let Some(username) = connect_msg.get_client_id() {
                self.add_user(&stream, username, &connect_msg)?;
                self.handle_connection(username, &mut stream)?;                
                println!("   se ha salido de la función, se deja de leer para el cliente: {}.", username); // debug
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
        println!("   tipo publish: Enviado el ack: {:?}", ack);
        Ok(())
    }

    ///Agrega el mensaje a la cola de mensajes de los usuarios suscritos al topic del mensaje
    fn _add_message_to_subscribers_queue(&self, msg: &PublishMessage) -> Result<(), Error> {
        // inicio probando
        //self.publish_msgs_tx.send(*msg); // pero cómo le digo de qué user y topic es
        // dicen mis notas "y que el hilo ppal, que el pcsamiento de iterar x user lo haga el hilo que escribe.
        // fin probando
        if let Ok(mut connected_users) = self.connected_users.lock() {
            for user in connected_users.values_mut() {
                //connected_users es un hashmap con key=username y value=user
                let user_topics = user.get_topics();
                if user_topics.contains(&(msg.get_topic())) {
                    //si el usuario está suscrito al topic del mensaje
                    user.add_message_to_queue(msg.clone());
                }
            }
        }
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
                // Publish
                let msg = PublishMessage::from_bytes(msg_bytes)?;
                println!("   Mensaje publish completo recibido: {:?}", msg);

                //self.send_puback_to_outgoing(&msg, stream)?; // Lo envía por un channel, hacia el lhilo outgoing.
                self.send_puback(&msg, stream)?;

                // Si llega un publish, lo mando por el channel, del otro lado (el hilo que llama a handle_outgoing_connections)
                // se encargará de enviarlo al/los suscriptor/es que tenga el topic del mensaje en cuestión.
                /*if self.publish_msgs_tx.send(Box::new(msg)).is_err() {
                    //println!("Error al enviar el PublishMessage al hilo que los procesa.");
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Error al enviar el PublishMessage al hilo que los procesa.",
                    ));
                }*/
                self.handle_publish_message(&msg)?;
            }
            PacketType::Subscribe => {
                // Subscribe
                let msg = SubscribeMessage::from_bytes(msg_bytes)?;
                let return_codes = self.add_topics_to_subscriber(username, &msg)?;

                //self.send_suback_to_outgoing(return_codes, stream)?; // Lo manda por un channel, hacia hilo outgoinf.
                self.send_suback(return_codes, stream)?; // Lo manda por un channel, hacia hilo outgoinf.
            }
            PacketType::Puback => {
                // PubAck
                println!("Recibo mensaje tipo PubAck");
                // Entonces tengo el mensaje completo
                let msg = PubAckMessage::msg_from_bytes(msg_bytes)?;
                println!("   Mensaje pub ack completo recibido: {:?}", msg);
            }
            _ => println!(
                "   ERROR: tipo desconocido: recibido: \n   {:?}",
                fixed_header
            ),
        };

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
                // ToDo: Leer de la doc qué hacer en este caso, o si solo ignoramos. []
            }
        };
        Ok(())
    }

    fn clone_ref(&self) -> Self {
        Self {
            connected_users: self.connected_users.clone(),
            available_packet_id: self.available_packet_id,
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

    /*/// Recibe los `PublishMessage`s recibidos de los clientes que le envía el otro hilo, y por cada uno,
    /// lo agrega a la queue de cada suscriptor y lo envía.
    fn handle_outgoing_messages(&self, rx: Receiver<Box<dyn Message + Send>>) -> Result<(), Error> {
        /* Aux: Estaba así, cuando era un Receiver<PublishMessage>
        while let Ok(msg_bytes) = rx.recv() {
            self.handle_publish_message(msg)?;
        }*/
        while let Ok(msg) = rx.recv() {

            match msg.get_type() {
                3 => {
                    if let Some(pub_msg) = msg.as_any().downcast_ref::<PublishMessage>() {
                        self.handle_publish_message(pub_msg)?;
                    }
                },
                /* // Aux: [] Restaba resolver cuál es el stream, acomodar para poder saber desde este hilo cuál es el stream por el que enviar.
                4 => {
                    if let Some(pub_ack_msg) = msg.as_any().downcast_ref::<PubAckMessage>() {
                        write_message_to_stream(&pub_ack_msg.to_bytes(), stream); // se puede wrappear en algo que maneje error y alguna cosa más.
                    }

                },
                9 => {
                    if let Some(sub_ack_msg) = msg.as_any().downcast_ref::<SubAckMessage>() {
                        write_message_to_stream(&sub_ack_msg.to_bytes(), stream);
                    }
                },*/
                _ => {

                }
            }
        }

        // Fin probando
        Ok(())
    }*/

    // Aux: el lock actualmente lo usa solo este hilo, por lo que "sobra". Ver más adelante si lo borramos (hacer tmb lo de los acks).
    /// Maneja los mensajes salientes, envía los mensajes a los usuarios conectados.
    fn handle_publish_message(&self, msg: &PublishMessage) -> Result<(), Error> {
        // Inicio probando
        // Acá debemos procesar el publish message: determinar a quiénes se lo debo enviar, agregarlo a su queue, y enviarlo.
        if let Ok(mut connected_users) = self.connected_users.lock() {
            for user in connected_users.values_mut() {
                //connected_users es un hashmap con key=username y value=user
                // User/s que se suscribió/eron al topic del PublishMessage:
                let user_topics = user.get_topics();
                if user_topics.contains(&(msg.get_topic())) {
                    //si el usuario está suscrito al topic del mensaje
                    user.add_message_to_queue(msg.clone());
                    // Aux: y acá mismo llamar al write que se lo mande.
                    // aux: el write actualmente hace pop. No tiene sentido hacer add message to queu
                    // aux: y en la línea siguiente hacerle pop, pero capaz sí tiene sentido que esté en la queue
                    // aux: por tema desconexiones. Ver [].
                }
            }
        }

        // Aux: se debe desencolar solamente cuando llega el ack. Por eso está en un for separado.
        // Envía
        self.pop_and_write()?;

        Ok(())
    }

    fn pop_and_write(&self) -> Result<(), Error> {
        // Aux: esto recorre todos los users, todos los topic, y hace pop de un msg de la queue.
        // aux: eso era xq antes no sabía qué se insertó a cada queue, por hacerlo un hilo diferente;
        // aux: actualmente sabemos xq lo hace el mismo hilo, pero tiene sentido que quede en la queue x tema desconexiones.
        if let Ok(connected_users) = self.connected_users.lock() {
            for user in connected_users.values() {
                if user.is_not_disconnected() {
                    let mut user_stream = user.get_stream()?;
                    let user_subscribed_topics = user.get_topics(); // Aux: (Esto sobra, las voy a recorrer a todas...)
                                                                    // println!("TOPICS: {:?}",topics);
                    let hashmap_messages = user.get_hashmap_messages();
                    if let Ok(mut hashmap_messages_locked) = hashmap_messages.lock() {
                        for topic in user_subscribed_topics {
                            // trae 1 cola de un topic y escribe los mensajes en el stream
                            if let Some(messages_topic_queue) = hashmap_messages_locked.get_mut(topic) {
                                while let Some(msg) = messages_topic_queue.pop_front() {
                                    let msg_bytes = msg.to_bytes();
                                    write_message_to_stream(&msg_bytes, &mut user_stream)?;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Crea un servidor en la dirección ip y puerto especificados.
fn create_server(ip: String, port: u16) -> Result<TcpListener, Error> {
    let listener =
        TcpListener::bind(format!("{}:{}", ip, port)).expect("Error al enlazar el puerto");
    Ok(listener)
}
