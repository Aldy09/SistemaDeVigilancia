use crate::mqtt::client::{
    mqtt_client_listener::MQTTClientListener, mqtt_client_retransmitter::Retransmitter,
    mqtt_client_server_connection::mqtt_connect_to_broker, mqtt_client_writer::MessageCreator,
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
        let writer = MessageCreator::new();
        let (publish_msg_tx, publish_msg_rx) = mpsc::channel::<PublishMessage>();
        let (retransmitter, ack_tx) = Retransmitter::new(stream.try_clone()?);
        let mut listener = MQTTClientListener::new(stream.try_clone()?, publish_msg_tx, ack_tx);

        let mqtt_client = MQTTClient {
            msg_creator: writer,
            retransmitter,
        };

        let listener_handle = thread::spawn(move || {
            let _ = listener.read_from_server();
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
        println!("[DEBUG TEMA ACK]: [CLIENT]: Por hacer publish:");
        // Esto solamente crea y devuelve el mensaje
        let msg = self.msg_creator.create_publish_msg(topic, payload, qos)?;
        println!("[DEBUG TEMA ACK]: [CLIENT]: Por esperar el ack:");
        // Se lo paso al retransmitter y que él se encargue de mandarlo, y retransmitirlo si es necesario
        self.retransmitter.send_and_retransmit(&msg)?;
        println!("[DEBUG TEMA ACK]: [CLIENT]: fin de la función, return a app.");

        Ok(msg)
    }

    /// Función de la librería de MQTTClient para realizar un subscribe.
    pub fn mqtt_subscribe(&mut self, topics: Vec<(String, u8)>) -> Result<(), Error> {
        println!("[DEBUG TEMA ACK]: [CLIENT]: Por hacer publish:");
        // Esto solamente crea y devuelve el mensaje
        let msg = self.msg_creator.create_subscribe_msg(topics)?;
        println!("[DEBUG TEMA ACK]: [CLIENT]: Por esperar el ack:");
        // Se lo paso al retransmitter y que él se encargue de mandarlo, y retransmitirlo si es necesario
        self.retransmitter.send_and_retransmit(&msg)?;
        println!("[DEBUG TEMA ACK]: [CLIENT]: fin de la función, return a app.");

        Ok(())
    }

    // ---------------
    // aux pensando: este esquema así de que sea una función llamada desde acá, es como llamarlo on demand
    // (aux:) cada vez que se hace un publish (o un subcribe), okey. Se inicia el tiempo cada vez que llega un publish.
    // (aux:) lo cual tiene sentido si mandamos de a uno y no mandamos más hasta que recibamos el ack. Ok.
    // Muevo lo que había acá para el retransmmitter, xq igual el retransmitter tmb necesita el Publish.

    // Aux: P/D, nota para el grupo: Comenté el listener xq dsp de mover esta parte a un Retransmitter para que quede más prolijo
    // aux: el listener quedaba sin usar, y tiene sentido, el listener es el del loop de fixed header y eso, no necstamos hablarle.
    // aux: (Así que yo hasta borraría el atributo listener y listo, total es una parte interna pero está bien, la lanzamos y desde afuera le hacen join al handle todo bien)
    // (Aux: P/D2: ah che una re pavada, se llama handLE y no handLER lo que devuelve el thread spawn, xD).
    // ------------------------------

    /// Función de la librería de MQTTClient para terminar de manera voluntaria la conexión con el server.
    pub fn mqtt_disconnect(&mut self) -> Result<(), Error> {
        let msg = self.msg_creator.create_disconnect_msg()?;
        self.retransmitter.send_and_shutdown_stream(msg)?;
        Ok(())
    }
}
