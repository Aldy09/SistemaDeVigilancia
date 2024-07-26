use std::{io::{Error, ErrorKind}, str::from_utf8};

use super::app_type::AppType;

/// Representa el contenido del will_message que se enviará desde las apps, y
/// eventualmente llegará a sistema monitoreo.
#[derive(Debug, PartialEq)]
pub struct WillContent {
    app_type_identifier: AppType,
    id: u8,    
}

impl WillContent {
    pub fn new(app_type_identifier: AppType, id: u8) -> Self {
        Self {app_type_identifier, id }
    }

    pub fn get_app_type_identifier(&self) -> AppType {
        self.app_type_identifier
    }

    pub fn get_id(&self) -> u8 {
        self.id
    }

    pub fn to_str(&self) -> String {
        let string_app_type = self.app_type_identifier.to_str();
        format!("{}-{}",string_app_type, self.id)
    }

    pub fn will_content_from_string(string: &str) -> Result<Self, Error> {
        let trimmed = string.trim();
        if let Some((app_type_string, id_string)) = trimmed.split_once('-') {
            // Obtiene el app type
            let app_type_identifier = AppType::app_type_from_str(app_type_string)?;
            // obtiene el id
            if let Ok(id) = id_string.parse::<u8>(){
                return Ok(Self{ app_type_identifier, id });
            }
        }
        Err(Error::new(ErrorKind::InvalidData, "Error al decodear WillContent."))
    }

    /// Convierte un struct `AppWillContent` a bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id.to_be_bytes());
        
        //bytes.extend_from_slice(&(self.app_type_identifier.len() as u8).to_be_bytes());
        bytes.extend_from_slice(self.app_type_identifier.to_str().as_bytes());

        bytes
    }

    /// Obtiene un struct `AppWillContent` a partir de bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        
        let id = u8::from_be_bytes([bytes[0]]);

        let string = from_utf8(&bytes[1..])
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "error de decodificación."))?;
        let app_type_identifier = AppType::app_type_from_str(string)?;

        Ok(Self{app_type_identifier, id})
    }
}


#[cfg(test)]
mod test {
    use crate::mqtt::mqtt_utils::will_message_utils::app_type::AppType;

    use super::WillContent;

    #[test]
    fn test_app_will_content_to_and_from_bytes_works() {
        // pasada a bytes y reconstruida es igual al original
        let will_msg = WillContent::new(AppType::Cameras, 1);
        
        assert_eq!(will_msg, WillContent::from_bytes(will_msg.to_bytes()).unwrap());
    }
}