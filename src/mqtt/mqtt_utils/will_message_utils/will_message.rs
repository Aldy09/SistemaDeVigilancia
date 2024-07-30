/// Struct utilizado por el MQTTServer.
/// Contiene la información relacionada al will_message extraída del ConnectMessage.
/// Se almacena en un User del MQTTServer, y es necesaria para posteriormente construir el PublishMessage
/// a enviar a los suscriptores del will_topic.
#[derive(Debug, PartialEq)]
pub struct WillMessageAndTopic {
    will_message_content: String,
    will_topic: String,
    qos: u8,
    will_retain: u8,
}

impl WillMessageAndTopic {
    pub fn new(will_message_content: String, will_topic: String, qos: u8, will_retain: u8) -> Self {
        Self {will_message_content, will_topic, qos, will_retain }
    }

    pub fn get_will_msg_content(&self) -> String {
        String::from(&self.will_message_content)
    }
    pub fn get_will_topic(&self) -> String {
        String::from(&self.will_topic)
    }

    pub fn get_qos(&self) -> u8 {
        self.qos
    }
    pub fn get_will_retain(&self) -> u8 {
        self.will_retain
    }
}