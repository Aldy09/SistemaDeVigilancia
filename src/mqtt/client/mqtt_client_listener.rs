use std::sync::mpsc::Sender;

use std::io::{Error, ErrorKind};

use crate::mqtt::messages::{
    packet_type::PacketType, puback_message::PubAckMessage, publish_message::PublishMessage,
    suback_message::SubAckMessage,
};

use crate::mqtt::client::ack_message::ACKMessage;
use crate::mqtt::mqtt_utils::fixed_header::FixedHeader;
use crate::mqtt::mqtt_utils::utils::{
    get_fixed_header_from_stream, get_whole_message_in_bytes_from_stream, is_disconnect_msg,
    send_puback, shutdown,
};

use super::mqtt_client::ClientStreamType;

#[derive(Debug)]
pub struct MQTTClientListener {
    stream: ClientStreamType,
    client_tx: Sender<PublishMessage>,
    ack_tx: Sender<ACKMessage>,
}

impl MQTTClientListener {
    pub fn new(
        stream: ClientStreamType,
        client_tx: Sender<PublishMessage>,
        ack_tx: Sender<ACKMessage>,
    ) -> Self {
        MQTTClientListener {
            stream,
            client_tx,
            ack_tx,
        }
    }

    /// Función que ejecutará un hilo de MQTTClient, dedicado exclusivamente a la lectura.
    pub fn read_from_server(&mut self) -> Result<(), Error> {
        let mut fixed_header_info: ([u8; 2], FixedHeader);

        loop {
            match get_fixed_header_from_stream(&mut self.stream) {
                Ok(Some((fixed_h_buf, fixed_h))) => {
                    fixed_header_info = (fixed_h_buf, fixed_h);

                    // Caso se recibe un disconnect
                    if is_disconnect_msg(&fixed_header_info.1) {
                        println!("Mqtt cliente leyendo: recibo disconnect");
                        shutdown(&self.stream);
                        break;
                    }

                    self.read_a_message(&fixed_header_info)?; // esta función lee UN mensaje.
                }
                Ok(None) => {
                    println!("Se cerró la conexión con server.");
                    break;
                }
                Err(_) => todo!(),
            }
        }

        Ok(())
    }

    /// Función interna que lee un mensaje, analiza su tipo, y lo procesa acorde a él.
    /// Función interna que lee un mensaje, analiza su tipo, y lo procesa acorde a él.
    fn read_a_message(&mut self, fixed_header_info: &([u8; 2], FixedHeader)) -> Result<(), Error> {
        let (fixed_header_bytes, fixed_header) = fixed_header_info;
        let tipo = fixed_header.get_message_type();
        let msg_bytes = get_whole_message_in_bytes_from_stream(
            fixed_header,
            &mut self.stream,
            fixed_header_bytes,
        )?;

        match tipo {
            PacketType::Publish => self.handle_publish(msg_bytes)?,
            PacketType::Puback => self.handle_puback(msg_bytes)?,
            PacketType::Suback => self.handle_suback(msg_bytes)?,
            _ => {
                println!(
                    "   ERROR: tipo desconocido: recibido: \n   {:?}",
                    fixed_header
                );
                return Err(Error::new(ErrorKind::Other, "Tipo desconocido."));
            }
        };

        Ok(())
    }

    fn handle_publish(&mut self, msg_bytes: Vec<u8>) -> Result<(), Error> {
        println!("Mqtt cliente leyendo: RECIBO MENSAJE TIPO PUBLISH");
        let msg = PublishMessage::from_bytes(msg_bytes)?;
        send_puback(&msg, &mut self.stream)?;
        // Envía PublishMessage a la app
        match self.client_tx.send(msg) {
            Ok(_) => println!("Mqtt cliente leyendo: se envía por tx exitosamente."),
            Err(_) => println!("Mqtt cliente leyendo: error al enviar por tx."),
        };
        Ok(())
    }

    fn handle_puback(&self, msg_bytes: Vec<u8>) -> Result<(), Error> {
        let msg = PubAckMessage::msg_from_bytes(msg_bytes)?;
        // Avisa que llegó el ack
        match self.ack_tx.send(ACKMessage::PubAck(msg)) {
            Ok(_) => println!("PubAck enviado por tx exitosamente."),
            Err(_) => println!("Error al enviar PubAck por tx."),
        }
        Ok(())
    }

    fn handle_suback(&self, msg_bytes: Vec<u8>) -> Result<(), Error> {
        let msg = SubAckMessage::from_bytes(msg_bytes)?;
        // Avisa que llegó el ack
        match self.ack_tx.send(ACKMessage::SubAck(msg)) {
            Ok(_) => println!("SubAck enviado por tx exitosamente."),
            Err(_) => println!("Error al enviar SubAck por tx."),
        }
        Ok(())
    }
}

/*impl Clone for MQTTClientListener {
    fn clone(&self) -> Self {
        let stream = self.stream.try_clone().unwrap();
        let client_tx = self.client_tx.clone();
        let ack_tx = self.ack_tx.clone();
        MQTTClientListener { stream, client_tx , ack_tx }
    }
}*/
