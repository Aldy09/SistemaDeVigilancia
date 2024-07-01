#[derive(Debug, PartialEq, Copy, Clone)]
pub enum PacketType {
    Connect = 1,
    Connack = 2,
    Publish = 3,
    Puback = 4,
    Pubrec = 5,
    Pubrel = 6,
    Pubcomp = 7,
    Subscribe = 8,
    Suback = 9,
    Unsubscribe = 10,
    Unsuback = 11,
    Pingreq = 12,
    Pingresp = 13,
    Disconnect = 14,

    //Reserved 0 y 15
    Reserved = 0,
}

impl From<u8> for PacketType {
    fn from(value: u8) -> Self {
        match value {
            1 => return PacketType::Connect,
            2 => return PacketType::Connack,
            3 => return PacketType::Publish,
            4 => return PacketType::Puback,
            5 => return PacketType::Pubrec,
            6 => return PacketType::Pubrel,
            7 => return PacketType::Pubcomp,
            8 => return PacketType::Subscribe,
            9 => return PacketType::Suback,
            10 => return PacketType::Unsubscribe,
            11 => return PacketType::Unsuback,
            12 => return PacketType::Pingreq,
            13 => return PacketType::Pingresp,
            14 => return PacketType::Disconnect,

            _ => return PacketType::Reserved,
        };
    }
}
