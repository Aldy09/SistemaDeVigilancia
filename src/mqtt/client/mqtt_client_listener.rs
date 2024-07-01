use std::{net::TcpStream, sync::mpsc::Sender};

use std::io::{Error, ErrorKind};

use crate::mqtt::messages::connack_message::ConnackMessage;

use crate::mqtt::messages::packet_type::PacketType;
use crate::mqtt::messages::puback_message::PubAckMessage;
use crate::mqtt::messages::suback_message::SubAckMessage;

use crate::mqtt::messages::publish_message::PublishMessage;
use crate::mqtt::mqtt_utils::aux_server_utils::{
    get_fixed_header_from_stream, get_whole_message_in_bytes_from_stream, is_disconnect_msg,
    send_puback, shutdown,
};
use crate::mqtt::mqtt_utils::fixed_header::FixedHeader;

type StreamType = TcpStream;

#[derive(Debug)]
pub struct MQTTClientListener {
    stream: StreamType,
    client_tx: Sender<PublishMessage>,
}

impl MQTTClientListener {
    pub fn new(stream: StreamType, client_tx: Sender<PublishMessage>) -> Self {
        MQTTClientListener { stream, client_tx }
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
                        shutdown(&self.stream);
                        break;
                    }

                    self.read_a_message(&fixed_header_info)?; // esta función lee UN mensaje.
                }
                Ok(None) => {
                    println!("Se desconectó el server.");
                    break}
                Err(_) => todo!(),
            }
        }

        Ok(())
    }

    /// Función interna que lee un mensaje, analiza su tipo, y lo procesa acorde a él.
    fn read_a_message(&mut self, fixed_header_info: &([u8; 2], FixedHeader)) -> Result<(), Error> {
        // He leído bytes de un fixed_header, tengo que ver de qué tipo es.
        let (fixed_header_bytes, fixed_header) = fixed_header_info;
        // Soy client, siempre inicio yo la conexión, puedo recibir distintos tipos de mensaje.
        let tipo = fixed_header.get_message_type();
        let msg_bytes = get_whole_message_in_bytes_from_stream(
            fixed_header,
            &mut self.stream,
            fixed_header_bytes,
        )?;

        match tipo {
            PacketType::Connack => {
                // ConnAck
                println!("Mqtt cliente leyendo: recibo conn ack");

                // Entonces tengo el mensaje completo
                let msg = ConnackMessage::from_bytes(&msg_bytes)?; //
                println!("   Mensaje conn ack completo recibido: {:?}", msg);
            }
            PacketType::Publish => {
                // Publish
                println!("Mqtt cliente leyendo: RECIBO MENSAJE TIPO PUBLISH");
                // Esto ocurre cuando me suscribí a un topic, y server me envía los msjs del topic al que me suscribí
                let msg = PublishMessage::from_bytes(msg_bytes)?;

                // Le respondo un pub ack
                let _ = send_puback(&msg, &mut self.stream);

                // Envío por el channel para que le llegue al cliente real (app cliente)
                match self.client_tx.send(msg) {
                    Ok(_) => println!("Mqtt cliente leyendo: se envía por tx exitosamente."),
                    Err(_) => println!("Mqtt cliente leyendo: error al enviar por tx."),
                };
            }
            PacketType::Puback => {
                // PubAck
                println!("Mqtt cliente leyendo: recibo pub ack");

                let msg = PubAckMessage::msg_from_bytes(msg_bytes)?;
                println!("   Mensaje pub ack completo recibido: {:?}", msg);
            }
            PacketType::Suback => {
                // SubAck
                println!("Mqtt cliente leyendo: recibo sub ack");

                let msg = SubAckMessage::from_bytes(msg_bytes)?;
                println!("   Mensaje sub ack completo recibido: {:?}", msg);
            }

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
}

impl Clone for MQTTClientListener {
    fn clone(&self) -> Self {
        let stream = self.stream.try_clone().unwrap();
        let client_tx = self.client_tx.clone();
        MQTTClientListener { stream, client_tx }
    }
}
