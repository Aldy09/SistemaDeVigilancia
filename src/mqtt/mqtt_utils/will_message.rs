//use std::{io::{Error, ErrorKind}, str::from_utf8};

/// Representa el campo will_message que estará presente en el payload
/// del ConnectMessage si cada app de cliente decide enviar uno.
#[derive(Debug, PartialEq)]
pub struct WillMessageAndTopic {
    //disconnected_client_identifier: String,
    will_message: String,
    will_topic: String,
}

impl WillMessageAndTopic {
    pub fn new(will_message: String, will_topic: String) -> Self {
        Self {will_message, will_topic}
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