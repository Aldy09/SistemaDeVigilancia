use std::{io::{Error, ErrorKind}, mem::size_of, str::from_utf8};

use crate::publish_flags::PublishFlags;

#[derive(Debug, PartialEq)]
pub struct PublishMessage {
    // Fixed header
    flags: PublishFlags,
    // Variable header
    topic_name: String,
    packet_identifier: u16,
    properties: Vec<(u8, u8)>, // [] ver si lo vamos a usar, puedo enviar len 0 siempre
    // Payload
    payload: Vec<u8>, // el payload a publicar, bytes
}

impl PublishMessage {
    // El payload recibido son bytes, por ejemplo una "string".as_bytes()
    pub fn new(flags: PublishFlags, topic_name: String, packet_identifier: u16, payload: &[u8]) -> Self {
        Self { flags, topic_name, packet_identifier, properties: vec![], payload: payload.to_vec() }

    }
    
    pub fn to_bytes(&self) -> Vec<u8> {

        let mut msg_bytes: Vec<u8> = vec![];

        let byte_de_flags = self.flags.to_flags_byte();
        msg_bytes.extend(byte_de_flags.to_be_bytes());

        let remaining_len = self.remaining_length(); 
        msg_bytes.extend(remaining_len.to_be_bytes());
        println!("Mando byte de flags: {:?} y remaining len: {}", byte_de_flags, remaining_len);
        println!("Mando self: {:?}", self);

        msg_bytes.extend((self.topic_name.len() as u16).to_be_bytes());
        msg_bytes.extend(self.topic_name.as_bytes());
        msg_bytes.extend(self.packet_identifier.to_be_bytes());
        msg_bytes.extend((self.properties.len() as u8).to_be_bytes());
        msg_bytes.extend(&self.payload);

        msg_bytes
    }

    /// Devuelve la remaining_length tal como se la especifica por el protocolo, para ser enviada.
    /// Esta longitud es utilizada al convertir a bytes un SubscribeMessage con el formato adecuado.
    /// Posteriormente, ese resultado se utiliza al leer los bytes para calcular la longitud del payload.
    fn remaining_length(&self) -> u8 {
        let var_len = self.variable_header_length();
        let mut rem_len = var_len;
        rem_len += self.payload.len() as u8;
        rem_len
    }

    /// Devuelve la longitud de la sección `Variable header` del struct `PublishMessage`.
    /// Es una función auxiliar para calcular diversas longitudes necesarias para el pasaje a y de bytes.
    fn variable_header_length(&self) -> u8 {
        PublishMessage::variable_header_length_for_values(self.topic_name.len() as u16, self.properties.len() as u8)
    }

    /// Devuelve la longitud de la sección `Variable header` del struct `PublishMessage`.
    /// Es una función auxiliar para calcular diversas longitudes necesarias para el pasaje a y de bytes.
    fn variable_header_length_for_values(topic_name_len: u16, properties_len: u8) -> u8 {
        let mut var_len = 2;//1; // 1 byte de topic_name.len()
        var_len += topic_name_len as u8; // n bytes del valor de topic_name.len()
        var_len += 2; // 2 bytes de packet_identifier;
        var_len += properties_len; // m bytes de len del vector
        var_len += 1; // []
        var_len
    }

    /// Calcula y devuelve la longitud del payload, necesaria para poder leerlo.
    fn payload_length(topic_name: String, properties: Vec<(u8, u8)>, remaining_length: u8) -> u8 {
        remaining_length - PublishMessage::variable_header_length_for_values(topic_name.len() as u16, properties.len() as u8)
    }

    pub fn pub_msg_from_bytes(msg_bytes: Vec<u8>) -> Result<PublishMessage, Error> {
        let size_of_u8 = size_of::<u8>();
        let mut idx = 0;
        // Leo byte de flags
        let flags_byte = (&msg_bytes[0..size_of_u8])[0];
        idx += size_of_u8;
        // Leo byte de remaining_len
        let remaining_len = (&msg_bytes[idx..idx+size_of_u8])[0];
        idx += size_of_u8;
        // Leo u16 de topic_name.len()
        let size_of_u16 = size_of::<u16>();
        let topic_name_len = u16::from_be_bytes(msg_bytes[idx..idx+size_of_u16].try_into().map_err(|_| Error::new(ErrorKind::Other, "Error leyendo bytes publ msg."))?); // forma 1
        idx += size_of_u16;
        // Leo la string topic_name, usando la len leída
        let topic_name = from_utf8(&msg_bytes[idx..idx+(topic_name_len as usize)]).map_err(|_| Error::new(ErrorKind::Other, "Error leyendo string en publ msg."))?;
        idx += topic_name_len as usize;
        // Leo u16 packet_identifier
        let packet_identifier = u16::from_be_bytes(msg_bytes[idx..idx+size_of_u16].try_into().map_err(|_| Error::new(ErrorKind::Other, "Error leyendo bytes publ msg."))?); // forma 1
        idx += size_of_u16;
        // Leo un u8 properties.len()
        let properties: Vec<(u8, u8)> = vec![];
        let properties_len = (&msg_bytes[idx..idx+size_of_u8])[0];
        if properties_len != 0 {
            return Err(std::io::Error::new(ErrorKind::Other, ""));
        }
        idx += size_of_u8;
        // Calculo len, del payload que debo leer
        let flags_creadas = PublishFlags::from_flags_byte(&flags_byte)?;
        let payload_len = PublishMessage::payload_length(String::from(topic_name), properties, remaining_len);
        // Tengo que leer payload_len bytes, mi atributo guarda bytes
        let payload = &msg_bytes[idx..idx+payload_len as usize];

        let string = String::from(topic_name);
        let struct_interpretado = PublishMessage::new(flags_creadas, string, packet_identifier, payload);
        Ok(struct_interpretado)
    }
    pub fn get_packet_id(&self) -> u16 {
        self.packet_identifier
    }
}



#[cfg(test)]
mod test {
    use crate::publish_message::{PublishMessage, PublishFlags};

    #[test]
    fn test_1_publish_msg_se_pasa_a_bytes_y_se_interpreta_correctamente(){

        let flags = PublishFlags::new(0,0,0).unwrap();
        let topic = String::from("topic1");
        let msg = PublishMessage::new(flags, topic, 1, "hola".as_bytes() );

        let bytes_msg = msg.to_bytes();

        let msg_reconstruido = PublishMessage::pub_msg_from_bytes(bytes_msg);

        assert_eq!(msg_reconstruido.unwrap(), msg);
    }
}