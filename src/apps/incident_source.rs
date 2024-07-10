use std::io::{Error, ErrorKind};

/// Representa el origen en el que se generó el incidente:
/// puede ser `Manual`, si fue generado manualmente desde la ui de sistema de monitoreo;
/// o `Automated` si se generó automáticamente mediante inteligencia artificial en sistema cámaras.
#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub enum IncidentSource {
    Manual,
    Automated,
}

impl IncidentSource {
    pub fn to_byte(&self) -> [u8; 1] {
        match self {
            IncidentSource::Manual => 1_u8.to_be_bytes(),
            IncidentSource::Automated => 2_u8.to_be_bytes(),
        }
    }

    pub fn from_byte(byte: [u8; 1]) -> Result<Self, Error> {
        match u8::from_be_bytes(byte) {
            1 => Ok(IncidentSource::Manual),
            2 => Ok(IncidentSource::Automated),
            _ => Err(Error::new(
                ErrorKind::Other,
                "Origen de incidente no válido",
            )),
        }
    }
}

#[cfg(test)]
mod test {
    use super::IncidentSource;

    #[test]
    fn test_1_incident_source_to_and_from_bytes_works() {
        // Variante Manual pasada a bytes y reconstruida es igual a la original
        let src_m = IncidentSource::Manual;
        assert_eq!(src_m, IncidentSource::from_byte(src_m.to_byte()).unwrap());

        // Variante Automated pasada a bytes y reconstruida es igual a la original
        let src_a = IncidentSource::Automated;
        assert_eq!(src_a, IncidentSource::from_byte(src_a.to_byte()).unwrap());
    }
}