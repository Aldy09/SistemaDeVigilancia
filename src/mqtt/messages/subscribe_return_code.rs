use std::io::{Error, ErrorKind};

#[derive(Debug, Copy, Clone, PartialEq)]
// El copy y clone son usados para enviarlo as u16.
pub enum SubscribeReturnCode {
    QoS0 = 0x00,
    QoS1 = 0x01,
    QoS2 = 0x02,
    Failure = 0x80,
}
impl SubscribeReturnCode {
    /// Recibe un número u16 y 'lo convierte' a (devuelve) la variante del enum correspondiente.
    /// Utillizado al leer el `ret_code` desde bytes.
    pub fn from_bytes(ret_code: u16) -> Result<SubscribeReturnCode, Error> {
        match ret_code {
            0x00 => Ok(SubscribeReturnCode::QoS0),
            0x01 => Ok(SubscribeReturnCode::QoS1),
            0x02 => Ok(SubscribeReturnCode::QoS2),
            0x80 => Ok(SubscribeReturnCode::Failure),
            _ => Err(Error::new(
                ErrorKind::Other,
                "Error, subscribe returned code inválido.",
            )),
        }
    }
}
