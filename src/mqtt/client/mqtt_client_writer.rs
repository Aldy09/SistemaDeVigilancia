use crate::mqtt::messages::{
    disconnect_message::DisconnectMessage, publish_flags::PublishFlags,
    publish_message::PublishMessage, subscribe_message::SubscribeMessage,
};

use std::io::{Error, ErrorKind};

#[derive(Debug)]
pub struct MQTTClientWriter {
    available_packet_id: u16,
}

impl MQTTClientWriter {
    pub fn new() -> MQTTClientWriter {
        MQTTClientWriter {
            available_packet_id: 0,
        }
    }

    pub fn mqtt_publish(
        &mut self,
        topic: &str,
        payload: &[u8],
        qos: u8,
    ) -> Result<PublishMessage, Error> {
        println!("-----------------");
        let packet_id = self.generate_packet_id();
        // Creo un msj publish
        let flags = PublishFlags::new(0, qos, 0)?;
        let result = PublishMessage::new(flags, topic, Some(packet_id), payload);
        let publish_message = match result {
            Ok(msg) => {
                //println!("Mqtt publish: envío publish: \n   {:?}", msg);
                println!("Publish enviado, topic: {:?}, packet_id: {:?}", msg.get_topic(), msg.get_packet_id());
                msg
            }
            Err(e) => return Err(Error::new(ErrorKind::Other, e)),
        };

        Ok(publish_message)
    }

    // Nuestras apps clientes llamarán a esta función (los drones, etc)
    /// Función parte de la interfaz para uso de clientes del protocolo MQTT.
    /// Recibe el packet id, y un vector de topics a los cuales cliente desea suscribirse.
    pub fn mqtt_subscribe(
        &mut self,
        topics_to_subscribe: Vec<(String, u8)>,
    ) -> Result<SubscribeMessage, Error> {
        let packet_id = self.generate_packet_id();
        println!("-----------------");
        // Construyo subscribe
        let subscribe_msg = SubscribeMessage::new(packet_id, topics_to_subscribe);
        println!("Mqtt subscribe: enviando mensaje: \n   {:?}", subscribe_msg);

        Ok(subscribe_msg)
    }

    /// Envía mensaje disconnect, y cierra la conexión con el servidor.
    pub fn mqtt_disconnect(&mut self) -> Result<DisconnectMessage, Error> {
        let msg = DisconnectMessage::new();
        Ok(msg)
    }

    /// Devuelve el packet_id a usar para el siguiente mensaje enviado.
    /// Incrementa en 1 el atributo correspondiente, debido a la llamada anterior, y devuelve el valor a ser usado
    /// en el envío para el cual fue llamada esta función.
    fn generate_packet_id(&mut self) -> u16 {
        self.available_packet_id += 1;
        self.available_packet_id
    }
}
