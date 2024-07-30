use crate::mqtt::client::mqtt_client_listener::MQTTClientListener;
use crate::mqtt::client::mqtt_client_writer::MQTTClientWritter;
use crate::mqtt::mqtt_utils::will_message_utils::will_content::WillContent;
use std::io::Error;
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};

use crate::mqtt::messages::publish_message::PublishMessage;
use crate::mqtt::messages::subscribe_message::SubscribeMessage;

use super::mqtt_client_server_connection::{mqtt_connect_to_broker, MqttClientConnection};

type StreamType = TcpStream;

#[derive(Debug)]
pub struct MQTTClient {
    writer: MQTTClientWritter,
    //listener: MQTTClientListener,
}

impl MQTTClient {
    pub fn mqtt_connect_to_broker(
        client_id: &str,
        addr: &SocketAddr,
        will_msg_content: WillContent,
        will_topic: &str,
        will_qos: u8,
    ) -> Result<(Self, Receiver<PublishMessage>, JoinHandle<()>), Error> {
        // Efectúa la conexión al server
        let stream =
            mqtt_connect_to_broker(client_id, addr, will_msg_content, will_topic, will_qos)?;

        // Inicializa su listener y writer
        let writer = MQTTClientWritter::new(stream.try_clone()?);
        let (publish_msg_tx, publish_msg_rx) = mpsc::channel::<PublishMessage>();
        let mut listener = MQTTClientListener::new(stream.try_clone()?, publish_msg_tx);

        let mqtt_client = MQTTClient { writer };

        let listener_handler = thread::spawn(move || {
            let _ = listener.read_from_server();
        });

        Ok((mqtt_client, publish_msg_rx, listener_handler))
    }

    pub fn new(stream: StreamType, listener: MQTTClientListener) -> MQTTClient {
        let writer = MQTTClientWritter::new(stream.try_clone().unwrap());
        MQTTClient { writer } //, listener }
    }

    // Delega la llamada al método mqtt_publish del writer
    pub fn mqtt_publish(
        &mut self,
        topic: &str,
        payload: &[u8],
        qos: u8,
    ) -> Result<PublishMessage, Error> {
        self.writer.mqtt_publish(topic, payload, qos)
    }

    pub fn mqtt_subscribe(&mut self, topics: Vec<String>) -> Result<SubscribeMessage, Error> {
        self.writer.mqtt_subscribe(topics)
    }

    pub fn mqtt_disconnect(&mut self) -> Result<(), Error> {
        self.writer.mqtt_disconnect()
    }
}

impl Clone for MQTTClient {
    fn clone(&self) -> Self {
        //let listener = self.listener.clone();
        let writer = self.writer.clone();
        MQTTClient { writer } //, listener }
    }
}
