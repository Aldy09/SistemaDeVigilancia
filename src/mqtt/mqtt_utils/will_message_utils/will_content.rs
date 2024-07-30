use std::io::{Error, ErrorKind};

use super::app_type::AppType;

/// Representa el contenido del will_message que se enviará desde las apps, y
/// eventualmente llegará a sistema monitoreo.
#[derive(Debug, PartialEq)]
pub struct WillContent {
    app_type_identifier: AppType,
    id: Option<u8>,    
}

impl WillContent {
    pub fn new(app_type_identifier: AppType, id: Option<u8>) -> Self {
        Self {app_type_identifier, id }
    }

    pub fn get_app_type_identifier(&self) -> AppType {
        self.app_type_identifier
    }

    pub fn get_id(&self) -> Option<u8> {
        self.id
    }

    pub fn to_str(&self) -> String {
        let string_app_type = self.app_type_identifier.to_str();
        let len = self.id.is_some() as u8; // indica si hay que continuar para leer el id o si el mismo es none.
        if let Some(id) = self.id {
            format!("{}-{}-{}",string_app_type, len, id)
        } else {
            format!("{}-{}-{}",string_app_type, len, 0)
        }
    }

    pub fn will_content_from_string(string: &str) -> Result<Self, Error> {
        let trimmed = string.trim();
        let parts: Vec<&str> = trimmed.split('-').collect();
        if parts.len() == 3 {
            let app_type_id_string = parts[0];
            let len_string = parts[1];
            let id_string = parts[2];
            // Obtiene el app type
            let app_type_identifier = AppType::app_type_from_str(app_type_id_string)?;
            // obtiene el len
            if let Ok(len) = len_string.parse::<u8>(){
                // si había, obtiene el id; else lo crea con None
                if len==1 {
                    if let Ok(id) = id_string.parse::<u8>(){
                        return Ok(Self{ app_type_identifier, id: Some(id)});
                    }
                } else {
                    return Ok(Self{ app_type_identifier, id: None });

                }
            }
        }
        Err(Error::new(ErrorKind::InvalidData, "Error al decodear WillContent."))
    }
}


#[cfg(test)]
mod test {
    use crate::mqtt::mqtt_utils::will_message_utils::app_type::AppType;

    use super::WillContent;

    #[test]
    fn test_app_will_content_to_and_from_bytes_works() {
        // pasada a string y reconstruida es igual al original
        let will_msg = WillContent::new(AppType::Cameras, Some(1));
        
        assert_eq!(will_msg, WillContent::will_content_from_string(will_msg.to_str().as_str()).unwrap());
    }
}