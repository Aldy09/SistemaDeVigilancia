use std::{io::{Error, ErrorKind}, str::from_utf8};

/// Representa el campo will_message que estará presente en el payload
/// del ConnectMessage si cada app de cliente decide enviar uno.
#[derive(Debug, PartialEq)]
pub struct WillMessage {
    disconnected_client_identifier: String,
}

impl WillMessage {
    pub fn new(disconnected_client_identifier: String) -> Self {
        Self {disconnected_client_identifier}
    }

     /// Convierte un struct `WillMessage` a bytes.
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
    } 
}

#[cfg(test)]
mod test {
    use super::WillMessage;

    #[test]
    fn test_will_message_to_and_from_bytes_works() {
        // pasada a bytes y reconstruida es igual al original
        let will_msg = WillMessage::new(String::from("probando"));
        
        assert_eq!(will_msg, WillMessage::from_bytes(will_msg.to_bytes()).unwrap());
    }
}