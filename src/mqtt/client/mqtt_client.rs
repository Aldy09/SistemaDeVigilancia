use crate::mqtt::client::{
    mqtt_client_listener::MQTTClientListener,
    mqtt_client_server_connection::mqtt_connect_to_broker, mqtt_client_writer::MQTTClientWriter,
};
use crate::mqtt::messages::publish_message::PublishMessage;
use crate::mqtt::messages::subscribe_message::SubscribeMessage;
use crate::mqtt::mqtt_utils::will_message_utils::will_message::WillMessageData;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::{
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
};

use super::mqtt_client_retransmitter::MQTTClientRetransmitter;

#[derive(Debug)]
pub struct MQTTClient {
    writer: MQTTClientWriter,
    //listener: MQTTClientListener, 
    retransmitter: MQTTClientRetransmitter,
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
        // Inicializa sus partes internas
        let writer = MQTTClientWriter::new(stream.try_clone()?);
        let (publish_msg_tx, publish_msg_rx) = mpsc::channel::<PublishMessage>();
        let (retransmitter, ack_tx) = MQTTClientRetransmitter::new();
        let mut listener = MQTTClientListener::new(stream.try_clone()?, publish_msg_tx, ack_tx);

        //let mut listener_clone = listener.clone();

        let mqtt_client = MQTTClient {
            writer,
            //listener,
            retransmitter,
        };

        let listener_handle = thread::spawn(move || {
            let _ = listener.read_from_server();
        });

        Ok((mqtt_client, publish_msg_rx, listener_handle))
    }

    // Las siguientes funciones son wrappers, delegan la llamada al método del mismo nombre del writer.

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
                if let Err(e) = self.wait_for_ack(&msg, qos) {
                    println!("Error al esperar ack del publish: {:?}", e);
                    // Si no se pudo esperar el ack, se deberia reintentar el publish
                }
                Ok(msg)
            }
            Err(e) => Err(e),
        }
    }

    /// Espera a recibir el ack para el packet_id del mensaje `msg`.
    fn wait_for_ack(&mut self, msg: &PublishMessage, qos: u8) -> Result<(), Error> {
        if qos == 1 {
            let packet_id = msg.get_packet_id();
            if let Some(packet_id) = packet_id {
                 //self.listener.wait_for_ack(packet_id, &mut self.ack_rx) // viejo
                 self.retransmitter.wait_for_ack(packet_id) // nuevo.
                 // aux pensando: este esquema así de que sea una función llamada desde acá, es como llamarlo on demand
                 // (aux:) cada vez que se hace un publish (o un subcribe), okey. Se inicia el tiempo cada vez que llega un publish.
                 // (aux:) lo cual tiene sentido si mandamos de a uno y no mandamos más hasta que recibamos el ack. Ok.
            } else {
                 Err(Error::new(
                    ErrorKind::Other,
                    "No se pudo obtener el packet id del mensaje publish",
                ))
            }
        } else {
            Ok(())
            // Aux: P/D, nota para el grupo: Comenté el listener xq dsp de mover esta parte a un Retransmitter para que quede más prolijo
            // aux: el listener quedaba sin usar, y tiene sentido, el listener es el del loop de fixed header y eso, no necstamos hablarle.
            // aux: (Así que yo hasts borraría el atributo listener y listo, total es una parte interna pero está bien, la lanzamos y desde afuera le hacen join al handle todo bien)
            // (Aux: P/D2: ah che una re pavada, se llama handLE y no handLER lo que devuelve el thread spawn, xD).
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
