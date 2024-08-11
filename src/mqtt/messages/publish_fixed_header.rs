use crate::mqtt::messages::publish_flags::PublishFlags;

#[derive(Debug, Clone, PartialEq)]
pub struct FixedHeader {
    pub flags: PublishFlags,  // byte 1, incluye también al msg_type.
    pub remaining_length: u8, // byte 2
}
