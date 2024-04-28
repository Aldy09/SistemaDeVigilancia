use std::mem::size_of;

//#[allow(dead_code)]
#[derive(Debug)]
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
            msg_bytes.extend(topic.1.to_be_bytes());
        }
        
        //msg_bytes.extend(self.topic_filters.to_be_bytes());

        println!("Bytes generados: {:?}", msg_bytes);
        msg_bytes
    }
}

pub fn subs_msg_from_bytes(mgs_bytes: Vec<u8>) -> SubscribeMessage {
    println!("Leyendo, bytes: {:?}", mgs_bytes);
    let size_of_u8 = size_of::<u8>();
    // Leo u8 type
    let tipo = &mgs_bytes[0..size_of_u8];
    // Leo u8 flags
    let flags = &mgs_bytes[size_of_u8..2*size_of_u8];
    println!("Leí tipo: {:?}, y flags: {:?}", tipo, flags);
    // Leo u16 packet_id
    let size_of_u16 = size_of::<u8>();
    let mut idx = 3*size_of_u8;
    let packet_id = &mgs_bytes[idx..idx+size_of_u16];
    idx+=size_of_u16;

    // Leo en u16 la longitud del vector de topics
    let topics_vec_len = &mgs_bytes[idx..idx+size_of_u16];
    idx+=size_of_u16;

    println!("Leí packet_id: {:?}, y topics_vec_len: {:?}", packet_id, topics_vec_len);

    // Leo cada elemento del vector: primero la len de la string en u16
    // y luego el elemento, que será una tupla (String, u8)
    //for  va acá
    //for _ in 0..topics_vec_len {
        let elem_string_len = &mgs_bytes[idx..idx+size_of_u16];
        idx += size_of_u16;
        let elem_qos = &mgs_bytes[idx..idx+size_of_u8];
        idx+=size_of_u8;
    //}




    // Aux probando, para que compile
    let packet_id: u16 = 1;
    let topics_to_subscribe: Vec<(String, u8)> = vec![(String::from("topic1"),1)];
    return SubscribeMessage::new(packet_id, topics_to_subscribe); // [] aux
}

#[cfg(test)]
mod test {
    use crate::subscribe_message::SubscribeMessage;

    #[test]
    fn test_1_subscribe_msg_se_crea_con_tipo_y_flag_adecuados() {
        let packet_id: u16 = 1;
        let topics_to_subscribe: Vec<(String, u8)> = vec![(String::from("topic1"),1)];
        let subscribe_msg = SubscribeMessage::new(packet_id, topics_to_subscribe);

        // Estos valores siempre son 8 y 2 respectivamente, para este tipo de mensaje
        assert_eq!(subscribe_msg.packet_type, 8);
        assert_eq!(subscribe_msg.flags, 2);
    }
}