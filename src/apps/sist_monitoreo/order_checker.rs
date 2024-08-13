use std::{collections::HashMap, io::Error};

use crate::{
    apps::{
        apps_mqtt_topics::AppsMqttTopics, sist_camaras::camera::Camera,
        sist_dron::dron_current_info::DronCurrentInfo,
    },
    mqtt::messages::publish_message::PublishMessage,
};

/// Componente encargado de mantener el campo relacionado con el timestamp del último mensaje recibido,
/// y responder si un dado mensaje es o no más nuevo que el último registrado.
#[derive(Debug)]
pub struct OrderChecker {
    timestamp_by_topic: HashMap<(String, u8), u128>, // ((Topic, id), timestamp)
}
impl OrderChecker {
    /// Crea e inicializa un `OrderChecker`.
    pub fn new() -> Self {
        Self {
            timestamp_by_topic: HashMap::new(),
        }
    }

    /// Verifica y devuelve si el timestamp del `publish_msg` recibido es más nuevo que el último procesado.
    pub fn is_newest(&mut self, publish_msg: &PublishMessage) -> Result<bool, Error> {
        let msg_topic = publish_msg.get_topic();
        let payload = publish_msg.get_payload();
        let recvd_timestamp = publish_msg.get_timestamp();

        match AppsMqttTopics::topic_from_str(&msg_topic)? {
            AppsMqttTopics::DronTopic => {
                let current_info = DronCurrentInfo::from_bytes(payload)?;
                let id: u8 = current_info.get_id();
                self.update_timestamp_if_newest(msg_topic, id, recvd_timestamp)
            }
            AppsMqttTopics::CameraTopic => {
                let camera = Camera::from_bytes(&payload);
                let id: u8 = camera.get_id();
                self.update_timestamp_if_newest(msg_topic, id, recvd_timestamp)
            }
            _ => Ok(true),
        }
    }

    /// Si el timestamp recibido es más nuevo que el almacenado para ese topic y id ('emisor'), entonces actualiza el
    /// almacenado con el nuevo. y devuelve true. Caso contrario devuelve false.
    fn update_timestamp_if_newest(
        &mut self,
        msg_topic: String,
        id: u8,
        rcvd_timestamp: u128,
    ) -> Result<bool, Error> {
        // Genera la clave a partir del topic y el id
        let key = (msg_topic, id);
        // Intenta obtener el último timestamp para la clave dada, o lo inserta si no existe
        if let Some(last_timestamp) = self.timestamp_by_topic.get_mut(&key) {
            // Ya se había recibido mensajes de ese emisor
            /*println!(
                "Timestamp rec: {}, guardado: {}, para ID: {}",
                rcvd_timestamp, *last_timestamp, id
            );*/

            // Si el timestamp recibido es más nuevo, actualiza el valor y devuelve true
            if rcvd_timestamp > *last_timestamp {
                println!("Se actualiza el timestamp");
                *last_timestamp = rcvd_timestamp;
                return Ok(true);
            }
            // Si el mensaje recibido para un mismo ID es más viejo, devuelve false
            Ok(false)
        } else {
            // No se encontró, por lo que es el primer mensaje de ese emisor, por lo tanto es el más nuevo
            self.timestamp_by_topic.insert(key, rcvd_timestamp);
            Ok(true)
        }
    }
}

impl Default for OrderChecker {
    fn default() -> Self {
        Self::new()
    }
}
