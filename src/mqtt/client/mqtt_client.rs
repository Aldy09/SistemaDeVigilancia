use crate::mqtt::client::mqtt_client_listener::MQTTClientListener;
use crate::mqtt::client::mqtt_client_writer::MQTTClientWritter;
use std::io::Error;
use std::net::TcpStream;

use crate::mqtt::messages::publish_message::PublishMessage;
use crate::mqtt::messages::subscribe_message::SubscribeMessage;

type StreamType = TcpStream;

#[derive(Debug)]
pub struct MQTTClient {
    writer: MQTTClientWritter,
    listener: MQTTClientListener,
}

impl MQTTClient {
    pub fn new(stream: StreamType, listener: MQTTClientListener) -> MQTTClient {
        let writer = MQTTClientWritter::new(stream.try_clone().unwrap());
        MQTTClient { writer, listener }
    }

    // Delega la llamada al método mqtt_publish del writer
    pub fn mqtt_publish(&mut self, topic: &str, payload: &[u8]) -> Result<PublishMessage, Error> {
        self.writer.mqtt_publish(topic, payload)
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
        let listener = self.listener.clone();
        let writer = self.writer.clone();
        MQTTClient { writer, listener }
    }
}
