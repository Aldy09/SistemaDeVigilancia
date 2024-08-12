use crate::mqtt::messages::{
    disconnect_message::DisconnectMessage, publish_flags::PublishFlags,
    publish_message::PublishMessage, subscribe_message::SubscribeMessage,
};

use std::io::Error;

#[derive(Debug)]
pub struct MessageCreator {
    available_packet_id: u16,
}

impl MessageCreator {
    pub fn new() -> MessageCreator {
        MessageCreator {
            available_packet_id: 0,
        }
    }

    /// Crea y devuelve el PublishMessage.
    pub fn create_publish_msg(
        &mut self,
        topic: &str,
        payload: &[u8],
        qos: u8,
    ) -> Result<PublishMessage, Error> {
        let packet_id = self.generate_packet_id();
        // Creo un msj publish
        let flags = PublishFlags::new(0, qos, 0)?;
        let publish_msg = PublishMessage::new(flags, topic, Some(packet_id), payload)?;

        Ok(publish_msg)
    }

    /// Recibe un vector de topics a los cuales cliente desea suscribirse.
    /// Crea y devuelve el SubscribeMessage.
    pub fn create_subscribe_msg(
        &mut self,
        topics_to_subscribe: Vec<(String, u8)>,
    ) -> Result<SubscribeMessage, Error> {
        let packet_id = self.generate_packet_id();
        // Construyo subscribe
        let subscribe_msg = SubscribeMessage::new(packet_id, topics_to_subscribe);        

        Ok(subscribe_msg)
    }

    /// Crea y devuelve un DisconnectMessage.
    pub fn create_disconnect_msg(&mut self) -> Result<DisconnectMessage, Error> {
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

impl Default for MessageCreator {
    fn default() -> Self {
        Self::new()
    }
}
