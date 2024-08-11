/*//pub struct para manejar los flags de subscribe mqtt
#[derive(Debug)]
pub struct SubscribeFlags {
    pub qos: u8,
}

impl SubscribeFlags {
    pub fn new(qos: u8) -> Self {
        SubscribeFlags { qos }
    }
    /// Convierte los flags de subscribe en un byte
    pub fn to_flags_byte(&self) -> u8 {
        self.qos & 0b0000_0011 // Solo se toman los 2 bits menos signif, dado que qos solo puede ser 0, 1 o 2
    }
}*/
