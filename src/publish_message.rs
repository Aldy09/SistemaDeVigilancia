
pub struct PublishFlags {

}
impl PublishFlags {
    pub fn new() -> Self {

        PublishFlags {  }
    }
}
impl PublishFlags {
    fn to_bytes(&self) -> Vec<u8> { // [] ver
        vec![0] // probando, para que compile
    }
}

#[derive(Debug)]
pub struct PublishMessage<'a> {
    flags: u8,
    topic_name: String,
    packet_identifier: u16,
    properties: Vec<(u8, u8)>, // [] ver si lo vamos a usar, puedo enviar len 0 siempre
    payload: &'a[u8], // el payload a publicar, bytes
}

impl<'a> PublishMessage<'a> {
    pub fn new(flags: PublishFlags, topic_name: String, packet_identifier: u16, payload: &'a[u8]) -> Self {
        
        let byte_de_flags = flags.to_bytes();
        Self { flags: byte_de_flags[0], topic_name, packet_identifier, properties: vec![], payload }

    }
    // El payload recibido son bytes, por ejemplo una "string".as_bytes()
    pub fn to_bytes(&self) -> Vec<u8> {

        let mut msg_bytes: Vec<u8> = vec![];
        msg_bytes.extend(self.flags.to_be_bytes());

        let remaining_len = self.remaining_length(); 
        msg_bytes.extend(remaining_len.to_be_bytes());
        msg_bytes.extend(self.topic_name.len().to_be_bytes());
        msg_bytes.extend(self.topic_name.as_bytes());
        msg_bytes.extend(self.packet_identifier.to_be_bytes());
        msg_bytes.extend(self.properties.len().to_be_bytes());
        msg_bytes.extend(self.payload);

        msg_bytes
        
    }

    /// Devuelve la remaining_length tal como se la especifica por el protocolo, para ser enviada.
    /// Esta longitud es utilizada al leer un SubscribeMessage para calcular la longitud del payload.
    fn remaining_length(&self) -> u8 {
        let mut rem_len = 1; // 1 byte de topic_name.len()
        rem_len += self.topic_name.len(); // n bytes del valor de topic_name.len()
        rem_len += 2; // 2 bytes de packet_identifier;
        rem_len += self.properties.len(); // m bytes de len del vector
        rem_len as u8
    }
}

#[cfg(test)]
mod test {
    use crate::publish_message::{PublishMessage, PublishFlags};

    #[test]
    fn test_1_publish_msg_se_crea(){

        let flags = PublishFlags::new();//PublishFlags{};
        let topic = String::from("topic1");
        let _msg = PublishMessage::new(flags, topic, 1, "hola".as_bytes() );

        // Probando que compile  

    }
}
