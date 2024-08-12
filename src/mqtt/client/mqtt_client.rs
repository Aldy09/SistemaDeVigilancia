use crate::logging::string_logger::StringLogger;
use crate::mqtt::client::{
    mqtt_client_listener::MQTTClientListener, mqtt_client_retransmitter::Retransmitter,
    mqtt_client_server_connection::MqttClientConnector,
    mqtt_client_msg_creator::MessageCreator,
};
use crate::mqtt::messages::publish_message::PublishMessage;
use crate::mqtt::mqtt_utils::will_message_utils::will_message::WillMessageData;
use std::net::TcpStream;
use std::{
    io::Error,
    net::SocketAddr,
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
};

pub type ClientStreamType = TcpStream; // Aux: que solo lo use el cliente por ahora, para hacer refactor más fácil.

#[derive(Debug)]
pub struct MQTTClient {
    msg_creator: MessageCreator,
    retransmitter: Retransmitter,
    logger: StringLogger,
}

impl MQTTClient {
    /// Función de la librería de MQTTClient para conectarse al servidor.
    /// Devuelve el MQTTClient al que solicitarle los demás métodos, un rx por el que recibir los PublishMessages que
    /// se publiquen a los topics a los que nos suscribamos, y un joinhandle que debe ser 'esperado' para finalizar correctamente la ejecución.
    pub fn mqtt_connect_to_broker(
        client_id: String,
        addr: &SocketAddr,
        will: Option<WillMessageData>,
        logger: StringLogger,
    ) -> Result<(Self, Receiver<PublishMessage>, JoinHandle<()>), Error> {
        // Efectúa la conexión al server
        let stream = MqttClientConnector::mqtt_connect_to_broker(client_id, addr, will, logger.clone_ref())?;
        // Inicializa sus partes internas
        let writer = MessageCreator::new();
        let (publish_msg_tx, publish_msg_rx) = mpsc::channel::<PublishMessage>();
        let (retransmitter, ack_tx) = Retransmitter::new(stream.try_clone()?, logger.clone_ref());
        let mut listener = MQTTClientListener::new(stream.try_clone()?, publish_msg_tx, ack_tx);
        
        let logger_c = logger.clone_ref();
        let mqtt_client = MQTTClient {
            msg_creator: writer,
            retransmitter,
            logger,
        };

        let listener_handle = thread::spawn(move || {
            if let Err(e) = listener.read_from_server(){
                logger_c.log(format!("Error al leer, en read_from_server: {:?}", e));
            }
        });

        Ok((mqtt_client, publish_msg_rx, listener_handle))
    }

    /// Función de la librería de MQTTClient para realizar un publish.
    pub fn mqtt_publish(
        &mut self,
        topic: &str,
        payload: &[u8],
        qos: u8,
    ) -> Result<PublishMessage, Error> {
        // Esto solamente crea y devuelve el mensaje
        let msg = self.msg_creator.create_publish_msg(topic, payload, qos)?;
        // Se lo paso al retransmitter y que él se encargue de mandarlo, y retransmitirlo si es necesario
        self.retransmitter.send_and_retransmit(&msg)?;

        //println!("-----------------\n Mqtt: publish enviado: \n   {:?}", msg);
        self.logger.log(format!("-----------------\n Mqtt: publish enviado: \n   {:?}", msg));

        Ok(msg)
    }

    /// Función de la librería de MQTTClient para realizar un subscribe.
    pub fn mqtt_subscribe(&mut self, topics: Vec<(String, u8)>) -> Result<(), Error> {
        // Esto solamente crea y devuelve el mensaje
        let msg = self.msg_creator.create_subscribe_msg(topics)?;
        // Se lo paso al retransmitter y que él se encargue de mandarlo, y retransmitirlo si es necesario
        self.retransmitter.send_and_retransmit(&msg)?;
        
        println!("-----------------\n Mqtt: subscribe enviado: \n   {:?}", msg);
        self.logger.log(format!("-----------------\n Mqtt: subscribe enviado: \n   {:?}", msg));

        Ok(())
    }

    /// Función de la librería de MQTTClient para terminar de manera voluntaria la conexión con el server.
    pub fn mqtt_disconnect(&mut self) -> Result<(), Error> {
        let msg = self.msg_creator.create_disconnect_msg()?;
        self.retransmitter.send_and_shutdown_stream(msg)?;
        Ok(())
    }
}
