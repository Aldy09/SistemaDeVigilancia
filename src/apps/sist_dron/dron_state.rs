use std::io::{Error, ErrorKind};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum DronState {
    ExpectingToRecvIncident,
    RespondingToIncident, // analizando si se va a mover (se evalúa la condición de los dos más cercanos)
    MustRespondToIncident, // confirmado que se va a mover al incidente
    Flying,
    Mantainance,
    ManagingIncident, // llegó al incidente
    IncidentResolved,
}

impl DronState {
    pub fn to_byte(&self) -> [u8; 1] {
        match self {
            DronState::ExpectingToRecvIncident => 1_u8.to_be_bytes(),
            DronState::RespondingToIncident => 2_u8.to_be_bytes(),
            DronState::MustRespondToIncident => 3_u8.to_be_bytes(),
            DronState::Flying => 4_u8.to_be_bytes(),
            DronState::Mantainance => 5_u8.to_be_bytes(),
            DronState::ManagingIncident => 6_u8.to_be_bytes(),
            DronState::IncidentResolved => 7_u8.to_be_bytes(),
        }
    }

    pub fn from_byte(bytes: [u8; 1]) -> Result<Self, Error> {
        match u8::from_be_bytes(bytes) {
            1 => Ok(DronState::ExpectingToRecvIncident),
            2 => Ok(DronState::RespondingToIncident),
            3 => Ok(DronState::MustRespondToIncident),
            4 => Ok(DronState::Flying),
            5 => Ok(DronState::Mantainance),
            6 => Ok(DronState::ManagingIncident),
            7 => Ok(DronState::IncidentResolved),
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                "Estado de dron no válido",
            )),
        }
    }
}
