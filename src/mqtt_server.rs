use crate::connect_message::ConnectMessage;
use crate::file_helper::read_lines;
use crate::fixed_header::FixedHeader;
use crate::mqtt_server_client_utils::{
    get_fixed_header_from_stream, get_whole_message_in_bytes_from_stream, write_message_to_stream,
};

use crate::puback_message::PubAckMessage;
use crate::publish_message::PublishMessage;
use crate::suback_message::SubAckMessage;
use crate::subscribe_message::SubscribeMessage;
use crate::subscribe_return_code::SubscribeReturnCode; // Add the missing import
use std::collections::HashMap;
use std::env::args;
use std::io::{Error, ErrorKind};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::{self};
use std::time::Duration;

use std::path::Path;
use std::vec;

type ShareableStream = Arc<Mutex<TcpStream>>;
type ShHashmapType = Arc<Mutex<HashMap<String, Vec<ShareableStream>>>>;
type ShareableStreams = Arc<Mutex<Vec<ShareableStream>>>;

#[allow(dead_code)]
pub struct MQTTServer {
    streams: ShareableStreams,
    subs_by_topic: ShHashmapType,
}

impl MQTTServer {
    pub fn new(ip: String, port: u16) -> Result<Self, Error> {
        let mqtt_server = Self {
            streams: Arc::new(Mutex::new(vec![])),
            subs_by_topic: Arc::new(Mutex::new(HashMap::new())),
        };
        let listener = create_server(ip, port)?;

        // Creo estructura subs_by_topic a usar (es un "Hashmap<topic, vec de subscribers>")
        // No es único hilo! al subscribe y al publish en cuestión lo hacen dos clientes diferentes! :)
        let subs_by_topic: ShHashmapType = Arc::new(Mutex::new(HashMap::new()));

        mqtt_server.handle_incoming_connections(listener, subs_by_topic)?;

        Ok((mqtt_server))
    }

    fn process_connect(
        &self,
        fixed_header: &FixedHeader,
        stream: &Arc<Mutex<TcpStream>>,
        fixed_header_buf: &[u8; 2],
        subs_by_topic: &ShHashmapType,
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
            self.was_the_session_created_succesfully(connect_msg)?;

        write_message_to_stream(&connack_response, stream)?;
        println!("   tipo connect: Enviado el ack: {:?}", connack_response);

        if is_authentic {
            self.handle_connection(stream, subs_by_topic)?;
        } else {
            println!("   ERROR: No se pudo autenticar al cliente.");
        }

        Ok(())
    }

    fn is_guest_mode_active(&self, user: Option<&str>, passwd: Option<&str>) -> bool {
        user.is_none() && passwd.is_none()
    }

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

    fn was_the_session_created_succesfully(
        &self,
        connect_msg: ConnectMessage,
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

    fn handle_connection(
        &self,
        stream: &Arc<Mutex<TcpStream>>,
        subs_by_topic: &ShHashmapType,
    ) -> Result<(), Error> {
        println!("Server esperando mensajes.");
        let mut fixed_header_info = get_fixed_header_from_stream(stream)?;
        let ceros: &[u8; 2] = &[0; 2];
        let mut vacio = &fixed_header_info.0 == ceros;
        while !vacio {
            self.continue_with_conection(stream, subs_by_topic, &fixed_header_info)?; // esta función lee UN mensaje.
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

    fn distribute_to_subscribers(
        &self,
        msg: &PublishMessage,
        subs_by_topic: &ShHashmapType,
    ) -> Result<(), Error> {
        let topic = msg.get_topic();
        let msg_bytes = msg.to_bytes();
        if let Ok(subs_by_top) = subs_by_topic.lock() {
            if let Some(topic_subscribers) = subs_by_top.get(&topic) {
                println!(
                    "   Se encontraron {} suscriptores al topic {:?}",
                    topic_subscribers.len(),
                    topic
                );
                //println!("Debug 1, pre for");
                for subscriber in topic_subscribers {
                    write_message_to_stream(&msg_bytes, subscriber)?;
                    println!("      enviado mensaje publish a subscriber");
                }
                //println!("Debug 2, afuera del for");
            }
        }
        Ok(())
    }

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

    fn add_subscribers_to_topic(
        &self,
        msg: &SubscribeMessage,
        stream: &Arc<Mutex<TcpStream>>,
        subs_by_topic: &ShHashmapType,
    ) -> Result<Vec<SubscribeReturnCode>, Error> {
        let mut return_codes = vec![];

        for (topic, _qos) in msg.get_topic_filters() {
            return_codes.push(SubscribeReturnCode::QoS1);
            let topic_s = topic.to_string();

            // Guarda una referencia (arc clone) al stream, en el vector de suscriptores al topic en cuestión
            if let Ok(mut subs_b_t) = subs_by_topic.lock() {
                subs_b_t
                    .entry(topic_s)
                    .or_insert_with(Vec::new)
                    .push(stream.clone());
            }
            println!("   Se agregó el suscriptor al topic {:?}", topic);
        }
        Ok(return_codes)
    }

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
        stream: &Arc<Mutex<TcpStream>>,
        subs_by_topic: &ShHashmapType,
        fixed_header_info: &([u8; 2], FixedHeader),
    ) -> Result<(), Error> {
        let (fixed_header_bytes, fixed_header) = fixed_header_info;

        // Ahora sí ya puede haber diferentes tipos de mensaje.
        match fixed_header.get_message_type() {
            3 => {
                let msg = self.process_publish(fixed_header, stream, fixed_header_bytes)?;

                self.send_puback(&msg, stream)?;

                self.distribute_to_subscribers(&msg, subs_by_topic)?;
            }
            8 => {
                let msg = self.process_subscribe(fixed_header, stream, fixed_header_bytes)?;

                let return_codes = self.add_subscribers_to_topic(&msg, stream, subs_by_topic)?;

                self.send_suback(return_codes, stream)?;
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
        let (fixed_header_buf, fixed_header) = get_fixed_header_from_stream(stream)?;

        // El único tipo válido es el de connect, xq siempre se debe iniciar la comunicación con un connect.
        match fixed_header.get_message_type() {
            1 => {
                self.process_connect(
                    &fixed_header,
                    stream,
                    &fixed_header_buf,
                    &self.subs_by_topic,
                )?;
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
            subs_by_topic: self.subs_by_topic.clone(),
        }
    }

    pub fn handle_incoming_connections(
        &self,
        listener: TcpListener,
        subs_by_topic: ShHashmapType,
    ) -> Result<(), Error> {
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

        for handle in handles {
            if let Err(e) = handle.join() {
                eprintln!("Error al esperar el hilo: {:?}", e);
            }
        }

        Ok(())
    }

    fn add_stream_to_vec(&self, stream: ShareableStream) {
        if let Ok(mut streams) = self.streams.lock() {
            streams.push(stream);
        }
    }
}

fn create_server(ip: String, port: u16) -> Result<TcpListener, Error> {
    let listener =
        TcpListener::bind(format!("{}:{}", ip, port)).expect("Error al enlazar el puerto");
    Ok(listener)
}
