use std::io::{Error, ErrorKind};

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum ConnectReturnCode {
    ConnectionAccepted = 0x00,
    ProtocolError = 0x01,
    IdentifierRejected = 0x02,
    ServerUnavailable = 0x03,
    BadUsernameOrPassword = 0x04,
    NotAuthorized = 0x05,
    UnspecifiedError = 0x80,
}

impl ConnectReturnCode {
    pub fn to_byte(&self) -> [u8; 1] {
        match self {
            ConnectReturnCode::ConnectionAccepted => 0_u8.to_be_bytes(),
            ConnectReturnCode::ProtocolError => 1_u8.to_be_bytes(),
            ConnectReturnCode::IdentifierRejected => 2_u8.to_be_bytes(),
            ConnectReturnCode::ServerUnavailable => 3_u8.to_be_bytes(),
            ConnectReturnCode::BadUsernameOrPassword => 4_u8.to_be_bytes(),
            ConnectReturnCode::NotAuthorized => 5_u8.to_be_bytes(),
            ConnectReturnCode::UnspecifiedError => 0x80_u8.to_be_bytes(),
        }
    }
    pub fn from_byte(bytes: [u8; 1]) -> Result<Self, Error> {
        match u8::from_be_bytes(bytes) {
            0 => Ok(ConnectReturnCode::ConnectionAccepted),
            1 => Ok(ConnectReturnCode::ProtocolError),
            2 => Ok(ConnectReturnCode::IdentifierRejected),
            3 => Ok(ConnectReturnCode::ServerUnavailable),
            4 => Ok(ConnectReturnCode::BadUsernameOrPassword),
            5 => Ok(ConnectReturnCode::NotAuthorized),
            0x80 => Ok(ConnectReturnCode::UnspecifiedError),
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                "Estado de dron no válido",
            )),
        }
    }
}