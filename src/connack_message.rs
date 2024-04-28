use crate::connect_return_code::ConnectReturnCode;
#[derive(Debug)]
#[allow(dead_code)]
pub struct ConnAckMessage {
    session_present: bool,
    return_code: ConnectReturnCode,
}