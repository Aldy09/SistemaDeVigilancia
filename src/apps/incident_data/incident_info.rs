use std::io::Error;

use super::incident_source::IncidentSource;

/// Este struct se utiliza como clave en hashmaps para identificar a un Incident.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct IncidentInfo {
    inc_id: u8,
    src: IncidentSource,
}
impl IncidentInfo {
    pub fn new(inc_id: u8, src: IncidentSource) -> Self {
        Self {inc_id, src}
    }
    pub fn get_inc_id(&self) -> u8 {
        self.inc_id
    }
    pub fn get_src(&self) -> &IncidentSource {
        &self.src
    }

    /// Convierte un struct `IncidentSource` a bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[self.inc_id]);
        bytes.extend_from_slice(&self.src.to_byte());
        bytes
    }

    /// Obtiene un struct `IncidentSource` a partir de bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Option<Self>, Error> {
        let inc_id = u8::from_be_bytes([bytes[0]]);
        if inc_id == 0 {
            return Ok(None);
        }
        let src = IncidentSource::from_byte([bytes[1]])?;

        Ok(Some(Self {
            inc_id,
            src,
        }))
    }   
}

#[cfg(test)]
mod test {
    use super::{IncidentInfo, IncidentSource};


    #[test]
    fn test_1_incident_info_to_and_from_bytes_works() {
        // pasada a bytes y reconstruida es igual a la original
        let src = IncidentSource::Manual;
        let inc_info = IncidentInfo::new(18, src);
        
        assert_eq!(inc_info, IncidentInfo::from_bytes(inc_info.to_bytes()).unwrap().unwrap());
    }
}