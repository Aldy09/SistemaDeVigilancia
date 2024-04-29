use std::{mem::size_of, io::{Error, ErrorKind}, str::from_utf8};

#[derive(Debug, PartialEq)]
pub struct SubscribeMessage {
    packet_type: u8, //para subscribe siempre es 8(por protocolo mqtt)
    flags: u8,       //para subscribe siempre es 2(por protocolo mqtt)
    packet_identifier: u16,
    topic_filters: Vec<(String, u8)>, // (topic, qos)
}

impl SubscribeMessage {
    pub fn new(packet_id: u16, topics: Vec<(String,u8)>) -> Self {
        SubscribeMessage { packet_type: 8, flags: 2, packet_identifier: packet_id, topic_filters: topics }
    }

    /// Pasa un struct SubscribeMessage a bytes, usando big endian.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut msg_bytes = vec![];
        msg_bytes.extend(self.packet_type.to_be_bytes());
        msg_bytes.extend(self.flags.to_be_bytes());
        msg_bytes.extend(self.packet_identifier.to_be_bytes());

        // Envío la longitud del vector de los topic_filters
        //let topic_filters_len: u16 = u16::from(self.topic_filters.len());
        let topic_filters_len: u16 = self.topic_filters.len() as u16;
        msg_bytes.extend(topic_filters_len.to_be_bytes());
        // Ahora cada longitud (de la string) y elemento del vector topic_filters
        for topic in &self.topic_filters {
            
            let topic_str_len = topic.0.len() as u16;
            msg_bytes.extend(topic_str_len.to_be_bytes());
            msg_bytes.extend(topic.0.as_bytes());
            msg_bytes.extend(topic.1.to_be_bytes());
        }
        
        msg_bytes
    }
}

/// Recibe bytes, y los interpreta.
/// Devuelve un struct SubscribeMessage con los valores recibidos e interpretados.
pub fn subs_msg_from_bytes(msg_bytes: Vec<u8>) -> Result<SubscribeMessage, Error> {
    let size_of_u8 = size_of::<u8>();
    // Leo u8 type
    //let tipo = &msg_bytes[0..size_of_u8];
    let tipo = (&msg_bytes[0..size_of_u8])[0];
    // Leo u8 flags
    let flags = (&msg_bytes[size_of_u8..2*size_of_u8])[0];
    let mut idx = 2*size_of_u8;

    // Leo u16 packet_id
    let size_of_u16 = size_of::<u16>();
    //let packet_id = &msg_bytes[idx..idx+size_of_u16];
    let packet_id = u16::from_be_bytes(msg_bytes[idx..idx+size_of_u16].try_into().map_err(|_| Error::new(ErrorKind::Other, "Error leyendo bytes subs msg."))?); // forma 1
    //let packet_id = u16::from_be_bytes([msg_bytes[idx], msg_bytes[idx+size_of_u8]]); // forma 2
    idx+=size_of_u16;

    // Leo en u16 la longitud del vector de topics
    let topics_vec_len = u16::from_be_bytes([msg_bytes[idx], msg_bytes[idx+size_of_u8]]); // forma 2
    idx+=size_of_u16;

    // Leo cada elemento del vector: primero la len de la string en u16
    // y luego el elemento, que será una tupla (String, u8)
    let mut topics: Vec<(String, u8)> = vec![];
    for _ in 0..topics_vec_len {
        // Leo la string len
        //let elem_string_len = &msg_bytes[idx..idx+size_of_u16];
        let elem_string_len = u16::from_be_bytes([msg_bytes[idx], msg_bytes[idx+size_of_u8]]); // forma 2        
        idx += size_of_u16;
        // Leo la string, de tam "elem_string_len"
        let string_leida = from_utf8(&msg_bytes[idx..idx+(elem_string_len as usize)]).unwrap();
        idx += elem_string_len as usize;
        // Leo el u8
        let elem_qos = (&msg_bytes[idx..idx+size_of_u8])[0];
        idx+=size_of_u8;

        // Terminé de leer, agrego el elemento leído al vector de topics
        let elemento = (String::from(string_leida), elem_qos);
        topics.push(elemento);
    }
    
    let struct_interpretado = SubscribeMessage{ packet_type: tipo, flags, packet_identifier: packet_id, topic_filters: topics };
    println!("Creo struct interpretado desde bytes: {:?}", struct_interpretado);

    Ok(struct_interpretado)
}

#[cfg(test)]
mod test {
    use crate::subscribe_message::SubscribeMessage;

    use super::subs_msg_from_bytes;

    #[test]
    fn test_1_subscribe_msg_se_crea_con_tipo_y_flag_adecuados() {
        let packet_id: u16 = 1;
        let topics_to_subscribe: Vec<(String, u8)> = vec![(String::from("topic1"),1)];
        let subscribe_msg = SubscribeMessage::new(packet_id, topics_to_subscribe);

        // Estos valores siempre son 8 y 2 respectivamente, para este tipo de mensaje
        assert_eq!(subscribe_msg.packet_type, 8);
        assert_eq!(subscribe_msg.flags, 2);
    }

    #[test]
    fn test_2_subscribe_msg_se_pasa_a_bytes_y_se_interpreta_correctamente() {
        let packet_id: u16 = 1;
        let topics_to_subscribe: Vec<(String, u8)> = vec![(String::from("topic1"),1)];
        let subscribe_msg = SubscribeMessage::new(packet_id, topics_to_subscribe);

        let bytes_msg = subscribe_msg.to_bytes();
        

        let msg_reconstruido = subs_msg_from_bytes(bytes_msg);
        assert_eq!(msg_reconstruido.unwrap(), subscribe_msg);
    }

    #[test]
    fn test_3_subscribe_msg_se_pasa_a_bytes_e_interpreta_correctamente_con_varios_topics() {
        let packet_id: u16 = 1;
        let mut topics_to_subscribe: Vec<(String, u8)> = vec![(String::from("topic1"),1)];
        topics_to_subscribe.push((String::from("topic2"),0)); // agrego más topics al vector
        topics_to_subscribe.push((String::from("topic3"),1));
        let subscribe_msg = SubscribeMessage::new(packet_id, topics_to_subscribe);

        let bytes_msg = subscribe_msg.to_bytes();
        

        let msg_reconstruido = subs_msg_from_bytes(bytes_msg);
        assert_eq!(msg_reconstruido.unwrap(), subscribe_msg);
    }
}