use crate::mqtt::client::{
    mqtt_client_listener::MQTTClientListener,
    mqtt_client_server_connection::mqtt_connect_to_broker, mqtt_client_writer::MQTTClientWriter,
};
use crate::mqtt::messages::publish_message::PublishMessage;
use crate::mqtt::messages::subscribe_message::SubscribeMessage;
use crate::mqtt::mqtt_utils::will_message_utils::will_message::WillMessageData;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::sync::mpsc::channel;
use std::{
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
};

use super::ack_message::ACKMessage;

#[derive(Debug)]
pub struct MQTTClient {
    writer: MQTTClientWriter,
    listener: MQTTClientListener,
    ack_rx: Receiver<ACKMessage>,
}

impl MQTTClient {
    /// Función de la librería de MQTTClient para conectarse al servidor.
    /// Devuelve el MQTTClient al que solicitarle los demás métodos, un rx por el que recibir los PublishMessages que
    /// se publiquen a los topics a los que nos suscribamos, y un joinhandle que debe ser 'esperado' para finalizar correctamente la ejecución.
    pub fn mqtt_connect_to_broker(
        client_id: String,
        addr: &SocketAddr,
        will: Option<WillMessageData>,
    ) -> Result<(Self, Receiver<PublishMessage>, JoinHandle<()>), Error> {
        // Efectúa la conexión al server
        let stream = mqtt_connect_to_broker(client_id, addr, will)?;
        let (ack_tx, ack_rx) = channel::<ACKMessage>();
        // Inicializa su listener y writer
        let writer = MQTTClientWriter::new(stream.try_clone()?);
        let (publish_msg_tx, publish_msg_rx) = mpsc::channel::<PublishMessage>();
        let listener = MQTTClientListener::new(stream.try_clone()?, publish_msg_tx, ack_tx);

        let mut listener_clone = listener.clone();

        let mqtt_client = MQTTClient {
            writer,
            listener,
            ack_rx,
        };

        let listener_handler = thread::spawn(move || {
            let _ = listener_clone.read_from_server();
        });

        Ok((mqtt_client, publish_msg_rx, listener_handler))
    }

    // Las siguientes funciones son wrappes, delegan la llamada al método del mismo nombre del writer.

    /// Función de la librería de MQTTClient para realizar un publish.
    pub fn mqtt_publish(
        &mut self,
        topic: &str,
        payload: &[u8],
        qos: u8,
    ) -> Result<PublishMessage, Error> {
        let pub_res = self.writer.mqtt_publish(topic, payload, qos);
        match pub_res {
            Ok(msg) => {
                if let Err(e) = self.handle_ack(&msg, qos) {
                    println!("Error al esperar ack del publish: {:?}", e);
                    // Si no se pudo esperar el ack, se deberia reintentar el publish
                }
                Ok(msg)
            }
            Err(e) => Err(e),
        }
    }

    fn handle_ack(&mut self, msg: &PublishMessage, qos: u8) -> Result<(), Error> {
        if qos == 1 {
            let packet_id = msg.get_packet_id();
            if let Some(packet_id) = packet_id {
                 self.listener.wait_to_ack(packet_id, &mut self.ack_rx)
            } else {
                 Err(Error::new(
                    ErrorKind::Other,
                    "No se pudo obtener el packet id del mensaje publish",
                ))
            }
        } else {
            Ok(())
        }
    }

    /// Función de la librería de MQTTClient para realizar un subscribe.
    pub fn mqtt_subscribe(&mut self, topics: Vec<(String, u8)>) -> Result<SubscribeMessage, Error> {
        self.writer.mqtt_subscribe(topics)
    }

    /// Función de la librería de MQTTClient para terminar de manera voluntaria la conexión con el server.
    pub fn mqtt_disconnect(&mut self) -> Result<(), Error> {
        self.writer.mqtt_disconnect()
    }

    // pub fn wait_to_ack(&mut self, packet_id: u16) -> Result<(), Error> {
    //     self.listener.wait_to_ack(packet_id, &mut self.ack_rx)
    // }
}
