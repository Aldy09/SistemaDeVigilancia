/// Contiene la información relacionada al will_message extraída del ConnectMessage.
/// Se almacena en un User, y es necesaria para posteriormente construir el PublishMessage
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

     /*/// Convierte un struct `WillMessage` a bytes.
     pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(self.disconnected_client_identifier.len() as u16).to_be_bytes());
        bytes.extend_from_slice(&self.disconnected_client_identifier.as_bytes());
        bytes
    }

    /// Obtiene un struct `WillMessage` a partir de bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        
        let len = u16::from_be_bytes([bytes[0], bytes[1]]);
        let string = from_utf8(&bytes[2..])
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "error de decodificación."))?;
        let disconnected_client_identifier = String::from(string);

        Ok(Self{disconnected_client_identifier})
    } */
}

/*
#[cfg(test)]
mod test {
    use super::WillMessageAndTopic;

    #[test]
    fn test_will_message_to_and_from_bytes_works() {
        // pasada a bytes y reconstruida es igual al original
        let will_msg = WillMessageAndTopic::new(String::from("probando"), String::from("probando2"));
        
        assert_eq!(will_msg, WillMessageAndTopic::from_bytes(will_msg.to_bytes()).unwrap());
    }
}*/