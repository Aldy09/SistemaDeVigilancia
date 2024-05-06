/// Struct que contiene los primeros dos bytes de cualquier tipo de mensaje del protocolo MQTT.
/// El byte 1 contiene el tipo de mensaje en sus 4 bits más significativos,
/// y ceros o posiblemente flags (dependiendo del tipo de mensaje) en sus 4 bits menos significativos.
/// El byte 2 contiene la `remaining_length` que es la longitud de la porción restante del mensaje.
#[derive(Debug, PartialEq)]
pub struct FixedHeader {
    message_type: u8,     // byte 1
    remaining_length: u8, // byte 2
}

impl FixedHeader {
    pub const fn fixed_header_len() -> usize {
        2 // dos bytes
    }

    pub fn from_bytes(msg_bytes: Vec<u8>) -> Self {

        let tipo = u8::from_be_bytes([msg_bytes[0]]);
        let rem_len = u8::from_be_bytes([msg_bytes[1]]);

        Self { message_type: tipo, remaining_length: rem_len }
    }

    pub fn get_tipo(&self) -> u8 {
        self.message_type >> 4
    }

    pub const fn get_rem_len(&self) -> usize {
        self.remaining_length as usize
    }
}