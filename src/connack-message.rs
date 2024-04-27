#[derive(Debug)]
struct ConnackMessage {
    session_present: bool,
    return_code: ConnectReturnCode,
}