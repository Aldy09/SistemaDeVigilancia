#[derive(Debug)]
pub struct VariableHeader {
    pub connect_acknowledge_flags: u8, // byte 3 --> 0000_000X (X = 1 if session present)
    pub connect_return_code: u8,       // byte 4
}
