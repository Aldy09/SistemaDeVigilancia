use std::io::{Error, ErrorKind};

#[derive(Debug, PartialEq, Clone)]
pub enum IncidentState {
    ActiveIncident,
    ResolvedIncident,
}

impl IncidentState {
    pub fn to_byte(&self) -> [u8; 1] {
        match self {
            IncidentState::ActiveIncident => 1_u8.to_be_bytes(),
            IncidentState::ResolvedIncident => 2_u8.to_be_bytes(),
        }
    }

    pub fn from_byte(byte: [u8; 1]) -> Result<Self, Error> {
        match u8::from_be_bytes(byte) {
            1 => Ok(IncidentState::ActiveIncident),
            2 => Ok(IncidentState::ResolvedIncident),
            _ => Err(Error::new(
                ErrorKind::Other,
                "Estado de incidente no válido",
            )),
        }
    }
}
