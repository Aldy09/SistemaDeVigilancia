use crate::connect_return_code::ConnectReturnCode;
#[derive(Debug)]
pub struct ConnackMessage {
    session_present: bool,
    return_code: ConnectReturnCode,
}