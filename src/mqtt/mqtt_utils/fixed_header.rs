use crate::mqtt::messages::packet_type::PacketType;

/// Struct que contiene los primeros dos bytes de cualquier tipo de mensaje del protocolo MQTT.
/// El byte 1 contiene el tipo de mensaje en sus 4 bits más significativos,
/// y ceros o posiblemente flags (dependiendo del tipo de mensaje) en sus 4 bits menos significativos.
/// El byte 2 contiene la `remaining_length` que es la longitud de la porción restante del mensaje.
#[derive(Debug, PartialEq)]
pub struct FixedHeader {
    message_type_byte: u8, // byte 1, el tipo está en los 4 MSBits.
    remaining_length: u8,  // byte 2
}

impl FixedHeader {
    pub const fn fixed_header_len() -> usize {
        2 // dos bytes
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        vec![self.message_type_byte, self.remaining_length]
    }

    pub fn from_bytes(msg_bytes: Vec<u8>) -> Self {
        let tipo = u8::from_be_bytes([msg_bytes[0]]);
        let rem_len = u8::from_be_bytes([msg_bytes[1]]);

        Self {
            message_type_byte: tipo,
            remaining_length: rem_len,
        }
    }

    pub fn get_message_type_byte(&self) -> u8 {
        self.message_type_byte >> 4
    }

    pub fn get_message_type(&self) -> PacketType {
        PacketType::from(self.get_message_type_byte())
    }

    pub const fn get_rem_len(&self) -> usize {
        self.remaining_length as usize
    }

    pub fn is_not_null(&self) -> bool {
        !((self.message_type_byte == 0) & (self.remaining_length == 0))
    }
}
