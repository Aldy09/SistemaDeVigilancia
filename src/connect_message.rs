#[derive(Debug)]
pub struct FixedHeader {
    message_type: u8, // byte 1
    remaining_length: u8, // byte 2
}

#[derive(Debug)]
pub struct VariableHeader {
    protocol_name: [u8; 4], // bytes 1-4
    protocol_level: u8, // byte 6
    connect_flags: ConnectFlags, // byte 7
}

#[derive(Debug)]
pub struct ConnectFlags {
    username_flag: bool, // bit 7
    password_flag: bool, // bit 6
    will_retain: bool, // bit 5
    will_qos: u8, // bits 3-4
    will_flag: bool, // bit 2
    clean_session: bool, // bit 1
    reserved: bool, // bit 0
}

#[derive(Debug)]
pub struct Payload<'a> {
    client_id: &'a str,
    will_topic: Option<&'a str>,
    will_message: Option<&'a str>,
    username: Option<&'a str>,
    password: Option<&'a str>,
}

#[derive(Debug)]
pub struct ConnectMessage<'a> {
    fixed_header: FixedHeader,
    variable_header: VariableHeader,
    payload: Payload<'a>,
}

pub trait ToBytes {
    fn to_bytes(&self) -> Vec<u8>;
}