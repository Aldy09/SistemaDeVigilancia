use crate::messages::connect_flags::ConnectFlags;

#[derive(Debug, PartialEq)]
pub struct VariableHeader {
    pub protocol_name: [u8; 4],      // bytes 1-4
    pub protocol_level: u8,          // byte 6
    pub connect_flags: ConnectFlags, // byte 7
}
