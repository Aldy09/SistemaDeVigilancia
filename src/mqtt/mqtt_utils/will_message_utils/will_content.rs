use std::{io::{Error, ErrorKind}, str::from_utf8};

/// Representa el contenido del will_message que se enviará desde las apps, y
/// eventualmente llegará a sistema monitoreo.
#[derive(Debug, PartialEq)]
pub struct WillContent {
    app_type_identifier: String,
    id: u8,    
}

impl WillContent {
    pub fn new(app_type_identifier: String, id: u8) -> Self {
        Self {app_type_identifier, id }
    }

    pub fn get_app_type_identifier(&self) -> String {
        String::from(&self.app_type_identifier)
    }

    pub fn get_id(&self) -> u8 {
        self.id
    }

    /// Convierte un struct `AppWillContent` a bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.to_be_bytes());
        
        //bytes.extend_from_slice(&(self.app_type_identifier.len() as u8).to_be_bytes());
        bytes.extend_from_slice(&self.app_type_identifier.as_bytes());

        bytes
    }

    /// Obtiene un struct `AppWillContent` a partir de bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        
        let id = u8::from_be_bytes([bytes[0]]);

        //let len = u8::from_be_bytes([bytes[1]]);
        let string = from_utf8(&bytes[1..])
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "error de decodificación."))?;
        let app_type_identifier = String::from(string);


        Ok(Self{app_type_identifier, id})
    }
}


#[cfg(test)]
mod test {
    use super::WillContent;

    #[test]
    fn test_app_will_content_to_and_from_bytes_works() {
        // pasada a bytes y reconstruida es igual al original
        let will_msg = WillContent::new(String::from("probando"), 1);
        
        assert_eq!(will_msg, WillContent::from_bytes(will_msg.to_bytes()).unwrap());
    }
}