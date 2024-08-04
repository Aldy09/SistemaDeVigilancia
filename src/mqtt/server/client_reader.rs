use crate::mqtt::messages::connect_message::ConnectMessage;
//use crate::mqtt::mqtt_utils::fixed_header_version_2::FixedHeader as FixedHeaderV2;

use crate::mqtt::messages::packet_type::PacketType;
use crate::mqtt::mqtt_utils::utils::{
    get_fixed_header_from_stream, get_fixed_header_from_stream_for_conn,
    get_whole_message_in_bytes_from_stream, is_disconnect_msg, shutdown,
};

use crate::mqtt::mqtt_utils::fixed_header::FixedHeader;
use crate::mqtt::server::packet::{self, Packet};

use std::io::{Error, Read};
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use super::client_validator::AuthenticateClient;
use super::client_writer::ClientWriter;
use super::message_processor::MessageProcessor;
use super::mqtt_server_2::MQTTServer;

#[derive(Debug)]
pub struct ClientReader {
    stream: TcpStream,
    mqtt_server: MQTTServer,
}

impl ClientReader {
    pub fn new(stream: TcpStream, mqtt_server: MQTTServer) -> Result<ClientReader, Error> {
        Ok(ClientReader {
            stream,
            mqtt_server,
        })
    }

    /// Procesa los mensajes entrantes de un dado cliente.
    pub fn handle_client(&mut self) -> Result<(), Error> {
        // Leo un fixed header, deberá ser de un connect
        let (fixed_header_buf, fixed_header) =
            get_fixed_header_from_stream_for_conn(&mut self.stream)?;

        let fixed_header_info = (fixed_header_buf, fixed_header);
        let (fixed_header_buf, fixed_header) = (&fixed_header_info.0, &fixed_header_info.1);

        let authenticator = AuthenticateClient::new();

        // El único tipo válido es el de connect, xq siempre se debe iniciar la comunicación con un connect.
        match fixed_header.get_message_type() {
            PacketType::Connect => {
                let connect_msg =
                    get_connect_message(fixed_header, &mut self.stream, fixed_header_buf)?;
                if authenticator.is_it_a_valid_connection(
                    &connect_msg,
                    &mut self.stream,
                    &self.mqtt_server,
                )? {
                    if let Some(username) = connect_msg.get_user() {
                        self.handle_packets(username)?;
                    }
                }
            }
            _ => {
                println!("Error, el primer mensaje recibido DEBE ser un connect.");
                println!("   recibido: {:?}", fixed_header);
                println!("Cerrando la conexión.");
                shutdown(&self.stream);
            }
        };
        Ok(())
    }

    // recibe paquetes del broker para enviarlos a su TcpStream y viceversa
    pub fn handle_packets(&mut self, username: &String) -> Result<(), Error> {
        let (tx_1, rx_1) = std::sync::mpsc::channel::<Packet>();
        let (tx_2, rx_2) = std::sync::mpsc::channel::<Vec<u8>>();

        let message_processor = MessageProcessor::new(self.mqtt_server.clone_ref(), tx_2)?;
        let mut client_writer = ClientWriter::new(self.stream.try_clone()?)?;
        let mut handlers = Vec::<JoinHandle<()>>::new();
        let mut self_clone = self.clone_ref();
        let username_clone = username.to_owned();

        // Envia al hilo de abajo los mensajes (en bytes) que llegan al servidor
        handlers.push(std::thread::spawn(move || {
            let _ = self_clone.handle_stream(username_clone.as_str(), tx_1);
        }));

        // Recibe los mensajes en bytes para posteriormente decodificarlos
        handlers.push(std::thread::spawn(move || {
            let _ = message_processor.handle_packets(rx_1);
        }));

        // Espera por paquetes que se necesiten escribir en el stream del cliente.
        // Por ejemplo, un mensaje de CONNACK, o un mensaje de un cierto topic al que se suscribió.
        handlers.push(std::thread::spawn(move || {
            let _ = client_writer.send_packets_to_client(rx_2);
        }));

        for h in handlers {
            let _ = h.join();
        }

        Ok(())
    }

    // Espera por paquetes que llegan desde su TcpStream y los envía al Broker
    pub fn handle_stream(
        &mut self,
        username: &str,
        tx_1: Sender<Packet>,
    ) -> Result<(), Error> {
        let mut fixed_header_info: ([u8; 2], FixedHeader);
        println!("Mqtt cliente leyendo: esperando más mensajes.");

        loop {
            match get_fixed_header_from_stream(&mut self.stream) {
                Ok(Some((fixed_h_buf, fixed_h))) => {
                    fixed_header_info = (fixed_h_buf, fixed_h);

                    // Caso se recibe un disconnect
                    if is_disconnect_msg(&fixed_header_info.1) {
                        self.mqtt_server.publish_users_will_message(username)?;
                        self.mqtt_server.remove_user(username);
                        println!("Mqtt cliente leyendo: recibo disconnect");
                        shutdown(&self.stream);
                        break;
                    }

                    let packet = create_packet(&fixed_h, &mut self.stream, &fixed_h_buf, username)?;

                    tx_1.send(packet).unwrap();
                }
                Ok(None) => {
                    println!("Se desconectó el cliente: {:?}.", username);
                    self.mqtt_server
                        .set_user_as_temporally_disconnected(username)?;
                    self.mqtt_server.publish_users_will_message(username)?;
                    // Acá se manejaría para recuperar la sesión cuando se reconecte.
                    break;
                }
                Err(_) => todo!(),
            }
        }
        Ok(())
    }

    fn clone_ref(&self) -> Self {
        ClientReader {
            stream: self.stream.try_clone().unwrap(),
            mqtt_server: self.mqtt_server.clone_ref(),
        }
    }
}

fn create_packet(fixed_header: &FixedHeader,
        stream: &mut TcpStream,
        fixed_header_buf: &[u8; 2], username: &str) -> Result<Packet, Error> {
    let msg_bytes = get_message_in_bytes(fixed_header, stream, fixed_header_buf)?;
    let message_type = fixed_header.get_message_type();
    Ok(Packet::new(message_type, msg_bytes,username.to_string()))
}

fn get_connect_message(
    fixed_header: &FixedHeader,
    stream: &mut TcpStream,
    fixed_header_buf: &[u8; 2],
) -> Result<ConnectMessage, Error> {
    let msg_bytes = get_whole_message_in_bytes_from_stream(fixed_header, stream, fixed_header_buf)?;
    Ok(ConnectMessage::from_bytes(&msg_bytes))
}

fn get_message_in_bytes(
    fixed_header: &FixedHeader,
    stream: &mut TcpStream,
    fixed_header_buf: &[u8; 2],
) -> Result<Vec<u8>, Error> {
    let msg_bytes = get_whole_message_in_bytes_from_stream(fixed_header, stream, fixed_header_buf)?;
    Ok(msg_bytes)
}
