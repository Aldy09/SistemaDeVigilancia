#[derive(Debug)]
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