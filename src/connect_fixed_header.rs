#[derive(Debug)]
pub struct FixedHeader {
    pub message_type: u8, // byte 1
    pub remaining_length: u8, // byte 2
}