use crate::mqtt::messages::connect_message::ConnectMessage;
//use crate::mqtt::mqtt_utils::fixed_header_version_2::FixedHeader as FixedHeaderV2;

use crate::mqtt::messages::packet_type::PacketType;
use crate::mqtt::mqtt_utils::utils::{
    get_fixed_header_from_stream, get_fixed_header_from_stream_for_conn,
    get_whole_message_in_bytes_from_stream, is_disconnect_msg, shutdown,
};

use crate::mqtt::mqtt_utils::fixed_header::FixedHeader;
use crate::mqtt::server::packet::Packet;
use crate::mqtt::stream_type::StreamType;

use std::io::Error;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;

use super::client_authenticator::AuthenticateClient;
use super::message_processor::MessageProcessor;
use super::mqtt_server::MQTTServer;

#[derive(Debug)]
pub struct ClientReader {
    stream: StreamType,
    mqtt_server: MQTTServer,
}

impl ClientReader {
    pub fn new(stream: StreamType, mqtt_server: MQTTServer) -> Result<ClientReader, Error> {
        Ok(ClientReader {
            stream,
            mqtt_server,
        })
    }

    /// Procesa los mensajes entrantes de un dado cliente.
    pub fn handle_client(&mut self, stream: &mut StreamType) -> Result<(), Error> {
        let (fixed_header_buf, fixed_header) = self.read_and_validate_header(stream)?;

        let authenticator = AuthenticateClient::new();
        self.authenticate_and_handle_connection(&fixed_header, &fixed_header_buf, &authenticator, stream)
    }

    fn read_and_validate_header(&mut self, stream: &mut StreamType) -> Result<([u8; 2], FixedHeader), Error> {
        let (fixed_header_buf, fixed_header) =
            get_fixed_header_from_stream_for_conn(stream)?;
        Ok((fixed_header_buf, fixed_header))
    }

    fn authenticate_and_handle_connection(
        &mut self,
        fixed_header: &FixedHeader,
        fixed_header_buf: &[u8; 2],
        authenticator: &AuthenticateClient,
        stream: &mut StreamType
    ) -> Result<(), Error> {
        match fixed_header.get_message_type() {
            PacketType::Connect => {
                let connect_msg =
                    get_connect_message(fixed_header, stream, fixed_header_buf)?;
                if authenticator.is_it_a_valid_connection(
                    &connect_msg,
                    stream,
                    &self.mqtt_server,
                )? {
                    // Aux: ok en realidad acá arriba al terminar el authenticator se crea el User. [].
                    // aux: o sea que a partir de acá (handle_packets) ya no debería usar el stream.
                    if let Some(client_id) = connect_msg.get_client_id() {
                        self.handle_packets(client_id)?;
                    }
                }
            }
            _ => self.handle_invalid_message(fixed_header, stream), // (aux: está ok xq no pasó por add_new_user).
        }
        Ok(())
    }

    fn handle_invalid_message(&self, fixed_header: &FixedHeader, stream: &mut StreamType) {
        println!("Error, el primer mensaje recibido DEBE ser un connect.");
        println!("   recibido: {:?}", fixed_header);
        println!("Cerrando la conexión.");
        shutdown(stream);
    }

    // Función modificada para usar las nuevas funciones modulares
    // Aux: dsp de lo de is_authentic, una vez que ya fue connect msg todo bien, viene esto:
    fn handle_packets(&mut self, client_id: &String) -> Result<(), Error> {
        let (tx_1, rx_1) = std::sync::mpsc::channel::<Packet>();

        let mut handles = Vec::<JoinHandle<()>>::new();

        // Hilo para obtener los bytes que llegan al servidor en el stream
        handles.push(self.spawn_stream_handler(client_id.to_owned(), tx_1));

        // Hilo para manejar la recepción y procesamiento de mensajes
        handles.push(self.spawn_message_processor(rx_1));

        for h in handles {
            let _ = h.join();
        }

        Ok(())
    }

    // Hilo para obtener los bytes que llegan al servidor en el stream
    fn spawn_stream_handler(
        &self,
        client_id_clone: String,
        tx_1: Sender<Packet>,
    ) -> JoinHandle<()> {
        let mut self_clone = self.clone_ref();
        std::thread::spawn(move || {
            let _ = self_clone.read_packets_from_stream(client_id_clone.as_str(), tx_1);
        })
    }

    // Hilo para manejar la recepción y procesamiento de mensajes
    fn spawn_message_processor(&self, rx_1: Receiver<Packet>) -> JoinHandle<()> {
        let mut message_processor = MessageProcessor::new(self.mqtt_server.clone_ref());
        std::thread::spawn(move || {
            let _ = message_processor.handle_packets(rx_1);
        })
    }
    // Espera por paquetes que llegan desde su stream y los envia al hilo de arriba
    pub fn read_packets_from_stream(&mut self, client_id: &str, tx_1: Sender<Packet>) -> Result<(), Error> {
        println!("Mqtt cliente leyendo: esperando más mensajes.");

        loop {
            match get_fixed_header_from_stream(&mut self.stream) {
                Ok(Some((fixed_h_buf, fixed_h))) => {
                    if is_disconnect_msg(&fixed_h) {
                        self.handle_disconnect(client_id)?; // aux: llama a mqtt []
                        break;
                    }
                    // Completa la lectura del stream, y envía al otro hilo para ser procesado
                    self.handle_packet(fixed_h, fixed_h_buf, client_id, &tx_1)?;
                }
                Ok(None) => {
                    self.handle_client_disconnection(client_id)?; // aux: llama a mqtt []
                    break;
                }
                Err(_) => todo!(),
            }
        }
        Ok(())
    }

    /// Desconexión voluntaria.
    fn handle_disconnect(&mut self, client_id: &str) -> Result<(), Error> {
        self.mqtt_server.publish_users_will_message(client_id)?;
        self.mqtt_server.remove_user(client_id);
        println!("Mqtt cliente leyendo: recibo disconnect");
        shutdown(&self.stream);
        Ok(())
    }

    fn handle_packet(
        &mut self,
        fixed_h: FixedHeader,
        fixed_h_buf: [u8; 2],
        client_id: &str,
        tx_1: &Sender<Packet>,
    ) -> Result<(), Error> {
        let packet = create_packet(&fixed_h, &mut self.stream, &fixed_h_buf, client_id)?;
        tx_1.send(packet).unwrap();
        Ok(())
    }

    /// Desconexión involuntaria (ie se le fue internet).
    fn handle_client_disconnection(&mut self, client_id: &str) -> Result<(), Error> {
        println!("Se desconectó el cliente: {:?}.", client_id);
        self.mqtt_server
            .set_user_as_temporally_disconnected(client_id)?; 
        self.mqtt_server.publish_users_will_message(client_id)?;
        Ok(())
    }

    fn clone_ref(&self) -> Self {
        ClientReader {
            stream: self.stream.try_clone().unwrap(),
            mqtt_server: self.mqtt_server.clone_ref(),
        }
    }
}

fn create_packet(
    fixed_header: &FixedHeader,
    stream: &mut StreamType, // []
    fixed_header_bytes: &[u8; 2],
    client_id: &str,
) -> Result<Packet, Error> {
    let msg_bytes = get_whole_message_in_bytes_from_stream(fixed_header, stream, fixed_header_bytes)?;
    let message_type = fixed_header.get_message_type();
    Ok(Packet::new(message_type, msg_bytes, client_id.to_string()))
}

/// Completa la lectura y devuelve el `ConnectMessage`.
fn get_connect_message(
    fixed_header: &FixedHeader,
    stream: &mut StreamType,
    fixed_header_bytes: &[u8; 2],
) -> Result<ConnectMessage, Error> {
    let msg_bytes = get_whole_message_in_bytes_from_stream(fixed_header, stream, fixed_header_bytes)?;
    Ok(ConnectMessage::from_bytes(&msg_bytes))
}
