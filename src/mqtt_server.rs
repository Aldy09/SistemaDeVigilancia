use crate::connected_user::User;
use crate::file_helper::read_lines;
use crate::fixed_header::FixedHeader;
use crate::messages::connect_message::ConnectMessage;
use crate::mqtt_server_client_utils::{
    get_fixed_header_from_stream, get_whole_message_in_bytes_from_stream, write_message_to_stream,
};

use crate::messages::puback_message::PubAckMessage;
use crate::messages::publish_message::PublishMessage;
use crate::messages::suback_message::SubAckMessage;
use crate::messages::subscribe_message::SubscribeMessage;
use crate::messages::subscribe_return_code::SubscribeReturnCode; // Add the missing import
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self};
use std::time::Duration;

use std::path::Path;
use std::vec;

type ShareableStream = Arc<Mutex<TcpStream>>;
type ShareableStreams = Arc<Mutex<Vec<ShareableStream>>>;
type ShareableUsers = Arc<Mutex<HashMap<String, User>>>;

#[allow(dead_code)]
pub struct MQTTServer {
    streams: ShareableStreams,
    connected_users: ShareableUsers,
    publish_msgs_tx: Sender<PublishMessage>, // rx no se puede clonar, mp"SC", pero el tx sí.
}

impl MQTTServer {
    pub fn new(ip: String, port: u16) -> Result<Self, Error> {
        //
        let (tx, rx) = mpsc::channel::<PublishMessage>();

        let mqtt_server = Self {
            streams: Arc::new(Mutex::new(vec![])),
            connected_users: Arc::new(Mutex::new(HashMap::new())),
            publish_msgs_tx: tx, // []
        };
        let listener = create_server(ip, port)?;

        let mqtt_server_hijo = mqtt_server.clone_ref();

        let incoming_thread = std::thread::spawn(move || {
            if let Err(result) = mqtt_server_hijo.handle_incoming_connections(listener) {
                println!("Error al manejar las conexiones entrantes: {:?}", result);
            }
        });

        let mqtt_server_hermano = mqtt_server.clone_ref();

        let outgoing_thread = std::thread::spawn(move || 
            if let Err(result) = mqtt_server_hermano.handle_outgoing_messages(rx) {
                println!("Error al manejar los mensajes salientes: {:?}", result);
        });

        incoming_thread
            .join()
            .expect("Failed to join incoming thread");
        outgoing_thread
            .join()
            .expect("Failed to join outgoing thread");

        Ok(mqtt_server)
    }

    /// Agrega un usuario al hashmap de usuarios conectados
    fn add_user(&self, stream: &Arc<Mutex<TcpStream>>, username: &str) {
        let user = User::new(stream.clone(), username.to_string());
        if let Ok(mut users) = self.connected_users.lock() {
            let username = user.get_username();
            users.insert(username, user); //inserta el usuario en el hashmap
        }
    }

    /// Procesa el mensaje de conexión recibido, autentica
    /// al cliente y envía un mensaje de conexión de vuelta.
    fn process_connect(
        &self,
        fixed_header: &FixedHeader,
        stream: &Arc<Mutex<TcpStream>>,
        fixed_header_buf: &[u8; 2],
    ) -> Result<(), Error> {
        // Continúa leyendo y reconstruye el mensaje recibido completo
        println!("Recibo mensaje tipo Connect");
        let msg_bytes = get_whole_message_in_bytes_from_stream(
            fixed_header,
            stream,
            fixed_header_buf,
            "connect",
        )?;
        let connect_msg = ConnectMessage::from_bytes(&msg_bytes);
        println!("Mensaje connect completo recibido: \n   {:?}", connect_msg);

        // Procesa el mensaje connect
        let (is_authentic, connack_response) =
            self.was_the_session_created_succesfully(&connect_msg)?;

        write_message_to_stream(&connack_response, stream)?;
        println!("   tipo connect: Enviado el ack: {:?}", connack_response);

        // Si el cliente se autenticó correctamente, se agrega a la lista de usuarios conectados(add_user)
        // y se maneja la conexión(handle_connection)
        if is_authentic {
            if let Some(username) = connect_msg.get_client_id() {
                self.add_user(stream, username);
                self.handle_connection(username, stream)?;
            }
        } else {
            println!("   ERROR: No se pudo autenticar al cliente.");
        }

        Ok(())
    }

    fn is_guest_mode_active(&self, user: Option<&str>, passwd: Option<&str>) -> bool {
        user.is_none() && passwd.is_none()
    }

    /// Autentica al usuario con las credenciales almacenadas en el archivo credentials.txt
    fn authenticate(&self, user: Option<&str>, passwd: Option<&str>) -> bool {
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
    ) -> Result<(bool, [u8; 4]), Error> {
        if self.is_guest_mode_active(connect_msg.get_user(), connect_msg.get_passwd())
            || self.authenticate(connect_msg.get_user(), connect_msg.get_passwd())
        {
            let connack_response: [u8; 4] = [0x20, 0x02, 0x00, 0x00]; // CONNACK (0x20) con retorno 0x00
            Ok((true, connack_response))
        } else {
            let connack_response: [u8; 4] = [0x20, 0x02, 0x00, 0x05]; // CONNACK (0x20) con retorno 0x05 (Refused, not authorized)
            Ok((false, connack_response))
        }
    }

    /// Maneja la conexión con el cliente, recibe mensajes y los procesa.
    fn handle_connection(
        &self,
        username: &str,
        stream: &Arc<Mutex<TcpStream>>,
    ) -> Result<(), Error> {
        // Inicio
        let mut fixed_header_info: ([u8; 2], FixedHeader);
        let ceros: &[u8; 2] = &[0; 2];
        let mut vacio: bool;

        //vacio = &fixed_header_info.0 == ceros;
        println!("Mqtt cliente leyendo: esperando más mensajes.");
        loop {
            if let Ok((fixed_h_buf, fixed_h)) = get_fixed_header_from_stream(&stream.clone()) {
                println!("While: leí bien.");
                // Guardo lo leído y comparo para siguiente vuelta del while
                fixed_header_info = (fixed_h_buf, fixed_h);
                vacio = &fixed_header_info.0 == ceros;
                break;
            };
            thread::sleep(Duration::from_millis(300)); // []
        }
        // Fin

        /*println!("Server esperando mensajes.");
        let mut fixed_header_info = get_fixed_header_from_stream(stream)?;
        let ceros: &[u8; 2] = &[0; 2];
        let mut vacio = &fixed_header_info.0 == ceros;
        */
        while !vacio {
            self.continue_with_conection(username, stream, &fixed_header_info)?; // esta función lee UN mensaje.
                                                                                 // Leo para la siguiente iteración
                                                                                 // Leo fixed header para la siguiente iteración del while, como la función utiliza timeout, la englobo en un loop
                                                                                 // cuando leyío algo, corto el loop y continúo a la siguiente iteración del while
            println!("Server esperando más mensajes.");
            loop {
                if let Ok((fixed_h, fixed_h_buf)) = get_fixed_header_from_stream(stream) {
                    // Guardo lo leído y comparo para siguiente vuelta del while
                    fixed_header_info = (fixed_h, fixed_h_buf);
                    vacio = &fixed_header_info.0 == ceros;
                    break;
                };
                thread::sleep(Duration::from_millis(300)); // []
            }
        }
        Ok(())
    }

    /// Procesa el mensaje de tipo Publish recibido.
    fn process_publish(
        &self,
        fixed_header: &FixedHeader,
        stream: &Arc<Mutex<TcpStream>>,
        fixed_header_bytes: &[u8; 2],
    ) -> Result<PublishMessage, Error> {
        println!("Recibo mensaje tipo Publish");
        let msg_bytes = get_whole_message_in_bytes_from_stream(
            fixed_header,
            stream,
            fixed_header_bytes,
            "publish",
        )?;
        let msg = PublishMessage::from_bytes(msg_bytes)?;
        println!("   Mensaje publish completo recibido: {:?}", msg);
        Ok(msg)
    }

    /// Envía un mensaje de tipo PubAck al cliente.
    fn send_puback(
        &self,
        msg: &PublishMessage,
        stream: &Arc<Mutex<TcpStream>>,
    ) -> Result<(), Error> {
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

    /// Procesa el mensaje de tipo Subscribe recibido.
    fn process_subscribe(
        &self,
        fixed_header: &FixedHeader,
        stream: &Arc<Mutex<TcpStream>>,
        fixed_header_bytes: &[u8; 2],
    ) -> Result<SubscribeMessage, Error> {
        println!("Recibo mensaje tipo Subscribe");
        let msg_bytes = get_whole_message_in_bytes_from_stream(
            fixed_header,
            stream,
            fixed_header_bytes,
            "subscribe",
        )?;
        let msg = SubscribeMessage::from_bytes(msg_bytes)?;

        Ok(msg)
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
        stream: &Arc<Mutex<TcpStream>>,
    ) -> Result<(), Error> {
        let ack = SubAckMessage::new(0, return_codes);
        let msg_bytes = ack.to_bytes();
        write_message_to_stream(&msg_bytes, stream)?;
        println!("   tipo subscribe: Enviado el ack: {:?}", ack);
        Ok(())
    }

    /// Se ejecuta una vez recibido un `ConnectMessage` exitoso y devuelto un `ConnAckMessage` acorde.
    /// Se puede empezar a recibir mensajes de otros tipos (`Publish`, `Subscribe`), de este cliente.
    /// Recibe el `stream` para la comunicación con el cliente en cuestión.
    /// Lee un mensaje.
    fn continue_with_conection(
        &self,
        username: &str,
        stream: &Arc<Mutex<TcpStream>>,

        fixed_header_info: &([u8; 2], FixedHeader),
    ) -> Result<(), Error> {
        let (fixed_header_bytes, fixed_header) = fixed_header_info;

        // Ahora sí ya puede haber diferentes tipos de mensaje.
        match fixed_header.get_message_type() {
            3 => {
                // Publish
                let msg = self.process_publish(fixed_header, stream, fixed_header_bytes)?;

                self.send_puback(&msg, stream)?;
                // println!(" Publish:  Antes de add_message_to_subscribers_queue");
                
                // Si llega un publish, lo mando por el channel, del otro lado (el hilo que llama a handle_outgoing_connections)
                // se encargará de enviarlo al/los suscriptor/es que tenga el topic del mensaje en cuestión.
                if self.publish_msgs_tx.send(msg).is_err() {
                    //println!("Error al enviar el PublishMessage al hilo que los procesa.");
                    return Err(Error::new(ErrorKind::Other, "Error al enviar el PublishMessage al hilo que los procesa."));
                }
                //self.add_message_to_subscribers_queue(&msg)?;
                // println!(" Publish:  Despues de add_message_to_subscribers_queue");
            }
            8 => {
                // Subscribe
                let msg = self.process_subscribe(fixed_header, stream, fixed_header_bytes)?;
                // println!(" Subscribe:  Antes de add_topics_to_subscriber");
                let return_codes = self.add_topics_to_subscriber(username, &msg)?;
                // println!(" Subscribe:  Despues de add_topics_to_subscriber");

                self.send_suback(return_codes, stream)?;
            }
            4 => {
                // PubAck
                println!("Recibo mensaje tipo PubAck");
                let msg_bytes = get_whole_message_in_bytes_from_stream(
                    fixed_header,
                    stream,
                    fixed_header_bytes,
                    "pub ack",
                )?;
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
    fn handle_client(&self, stream: &Arc<Mutex<TcpStream>>) -> Result<(), Error> {
        // Probando
        let fixed_header_info: ([u8; 2], FixedHeader);

        //vacio = &fixed_header_info.0 == ceros;
        println!("Mqtt cliente leyendo: esperando más mensajes.");
        loop {
            if let Ok((fixed_h_buf, fixed_h)) = get_fixed_header_from_stream(&stream.clone()) {
                println!("While: leí bien.");
                // Guardo lo leído y comparo para siguiente vuelta del while
                fixed_header_info = (fixed_h_buf, fixed_h);
                break;
            };
            thread::sleep(Duration::from_millis(300)); // []
        }
        // Fin Probando
        //let (fixed_header_buf, fixed_header) = get_fixed_header_from_stream(stream)?;
        let (fixed_header_buf, fixed_header) = (&fixed_header_info.0, &fixed_header_info.1);

        // El único tipo válido es el de connect, xq siempre se debe iniciar la comunicación con un connect.
        match fixed_header.get_message_type() {
            1 => {
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
            streams: self.streams.clone(),
            connected_users: self.connected_users.clone(),
            publish_msgs_tx: self.publish_msgs_tx.clone(), // []
        }
    }

    /// Maneja las conexiones entrantes, acepta las conexiones y crea un hilo para manejar cada una.
    pub fn handle_incoming_connections(&self, listener: TcpListener) -> Result<(), Error> {
        println!("Servidor iniciado. Esperando conexiones.");
        let mut handles = vec![];

        for stream_client in listener.incoming() {
            match stream_client {
                Ok(stream_client) => {
                    let stream_client_sh = Arc::new(Mutex::new(stream_client));
                    let self_hijo = self.clone_ref();
                    self_hijo.add_stream_to_vec(stream_client_sh.clone());
                    let handle = std::thread::spawn(move || {
                        let _ = self_hijo.handle_client(&stream_client_sh);
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

    /// Agrega un stream a la lista de streams.
    fn add_stream_to_vec(&self, stream: ShareableStream) {
        if let Ok(mut streams) = self.streams.lock() {
            streams.push(stream);
        }
    }

    /// Recibe los `PublishMessage`s recibidos de los clientes que le envía el otro hilo, y por cada uno,
    /// lo agrega a la queue de cada suscriptor y lo envía.
    fn handle_outgoing_messages(&self, rx: Receiver<PublishMessage>) -> Result<(), Error> {
        while let Ok(msg) = rx.recv() {
            self.handle_publish_message(msg)?;
        }
        Ok(())
    }

    // Aux: el lock actualmente lo usa solo este hilo, por lo que "sobra". Ver más adelante si lo borramos (hacer tmb lo de los acks).
    /// Maneja los mensajes salientes, envía los mensajes a los usuarios conectados.
    fn handle_publish_message(&self, msg: PublishMessage) -> Result<(), Error> {

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
                let stream = user.get_stream();
                let topics = user.get_topics();
                // println!("TOPICS: {:?}",topics);
                let hashmap_messages = user.get_messages();
                if let Ok(mut hashmap_messages_locked) = hashmap_messages.lock() {
                    for topic in topics {
                        // trae 1 cola de un topic y escribe los mensajes en el stream
                        if let Some(messages_topic_queue) = hashmap_messages_locked.get_mut(topic) {
                            while let Some(msg) = messages_topic_queue.pop_front() {
                                let msg_bytes = msg.to_bytes();
                                write_message_to_stream(&msg_bytes, &stream)?;
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
