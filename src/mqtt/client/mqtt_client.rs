use crate::mqtt::client::{
    mqtt_client_listener::MQTTClientListener, mqtt_client_retransmitter::MQTTClientRetransmitter,
    mqtt_client_server_connection::mqtt_connect_to_broker, mqtt_client_writer::MQTTClientWriter,
};
use crate::mqtt::messages::message::Message;
use crate::mqtt::messages::packet_type::PacketType;
use crate::mqtt::messages::{publish_message::PublishMessage, subscribe_message::SubscribeMessage};
use crate::mqtt::mqtt_utils::will_message_utils::will_message::WillMessageData;
use std::{
    io::{Error, ErrorKind},
    net::SocketAddr,
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
};

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
        println!("[DEBUG TEMA ACK]: [CLIENT]: Por hacer publish:");
        let msg = self.writer.mqtt_publish(topic, payload, qos)?;
        println!("[DEBUG TEMA ACK]: [CLIENT]: Por esperar el ack:");
        // Esperar ack y retransmitir si no llega
        if let Err(e) = self.wait_for_ack(msg.clone()) {
            println!("Error al esperar ack del publish: {:?}", e);
        };
        println!(
            "[DEBUG TEMA ACK]: [CLIENT]: fin de la función, packet_id: {:?}, return a app.",
            &msg.get_packet_id()
        );
        Ok(msg)
    }

    // Si no se pudo esperar el ack, se deberia reintentar el publish
    /// Espera a recibir el ack para el packet_id del mensaje `msg`.
    fn wait_for_ack<T: Message>(&mut self, msg: T) -> Result<(), Error> {
        match msg.get_type() {           
            PacketType::Publish => {
                msg.get_qos();
                /*// Si es publish, ver el qos
                if qos == 1 {
                    return self.wait_and_retransmit(msg):
                } else {
                    return Ok(());
                }*/
            },
            PacketType::Subscribe => todo!(),
            _ => {}
        }
        
        Ok(())
    }

    fn wait_and_retransmit<T: Message>(&self, msg: T) -> Result<(), Error> {
        let packet_id = msg.get_packet_id();
        // Espero la primera vez, para el publish que hicimos arriba. Si se recibió ack, no hay que hacer nada más.
        let mut received_ack = self.retransmitter.wait_for_ack(packet_id)?;
        if received_ack {
            return Ok(());
        }

        // No recibí ack, entonces tengo que continuar retransmitiendo, hasta un máx de veces.
        const AMOUNT_OF_RETRIES: u8 = 5; // cant de veces que va a reintentar, hasta que desista y dé error.
        let mut remaining_retries = AMOUNT_OF_RETRIES;
        while remaining_retries > 0 {
            // Si el Retransmitter determina que se debe volver a enviar el mensaje, lo envío.
            if !received_ack {
                self.writer.resend_msg(msg.to_bytes())?
            } else {
                break;
            }
            received_ack = self.retransmitter.wait_for_ack(packet_id)?;

            remaining_retries -= 1; // Aux: sí, esto podría ser un for. Se puede cambiar.
        }

        if !received_ack {
            // Ya salí del while, retransmití muchas veces y nunca recibí el ack, desisto
            return Err(Error::new(
                ErrorKind::Other,
                "MAXRETRIES, se retransmitió sin éxito.",
            ));
        }

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

    /// Función de la librería de MQTTClient para realizar un subscribe.
    pub fn mqtt_subscribe(&mut self, topics: Vec<(String, u8)>) -> Result<SubscribeMessage, Error> {
        //self.writer.mqtt_subscribe(topics)
        // Lo nuevo:
        println!("[DEBUG TEMA ACK]: [CLIENT]: Por hacer publish:");
        let msg = self.writer.mqtt_subscribe(topics)?;
        println!("[DEBUG TEMA ACK]: [CLIENT]: Por esperar el ack:");
        // Esperar ack y retransmitir si no llega
        if let Err(e) = self.wait_for_ack(msg) {
            println!("Error al esperar ack del publish: {:?}", e);
        };
        /*println!(
            "[DEBUG TEMA ACK]: [CLIENT]: fin de la función, packet_id: {:?}, return a app.",
            &msg.get_packet_id()
        );*/

        //Ok(msg)
        Err(Error::new(ErrorKind::Other, "probando")) // aux: borrar esta lína
    }

    /// Función de la librería de MQTTClient para terminar de manera voluntaria la conexión con el server.
    pub fn mqtt_disconnect(&mut self) -> Result<(), Error> {
        self.writer.mqtt_disconnect()
    }
}
