use std::io::{Error, ErrorKind};

#[derive(Debug, PartialEq)]
pub enum DronState {
    ExpectingToRecvIncident,
    RespondingToIncident,
    Mantainance,


}

impl DronState {
    pub fn to_byte(&self) -> [u8; 1] {
        match self {
            DronState::ExpectingToRecvIncident => 1_u8.to_be_bytes(),
            DronState::RespondingToIncident => 2_u8.to_be_bytes(),
            DronState::Mantainance => 3_u8.to_be_bytes(),
        }
    }

    pub fn from_byte(bytes: [u8; 1]) -> Result<Self, Error> {
        match u8::from_be_bytes(bytes) {
            1 => Ok(DronState::ExpectingToRecvIncident),
            2 => Ok(DronState::RespondingToIncident),
            3 => Ok(DronState::Mantainance),
            _ => Err(Error::new(ErrorKind::InvalidInput, "Estado de dron no válido")),
        }
    }
}
