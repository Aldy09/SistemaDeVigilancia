use crate::mqtt::messages::packet_type::PacketType;

pub struct Packet {
    message_type: PacketType,
    msg_bytes: Vec<u8>,
    username: String,
}

impl Packet {
    pub fn new(message_type: PacketType, msg_bytes: Vec<u8>, username: String) -> Packet {
        Packet {
            message_type,
            msg_bytes,
            username,
        }
    }

    pub fn get_message_type(&self) -> PacketType {
        self.message_type
    }

    pub fn get_msg_bytes(&self) -> Vec<u8> {
        self.msg_bytes.clone()
    }

    pub fn get_username(&self) -> &str {
        self.username.as_str()
    }
}
