
pub struct FixedHeader {
    pub message_type: u8,
    pub reserved: u8,
    pub remaining_length: u8,
}