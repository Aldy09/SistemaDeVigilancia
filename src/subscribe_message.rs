//#[allow(dead_code)]
#[derive(Debug)]
pub struct SubscribeMessage {
    pub packet_type: u8, //para subscribe siempre es 8(por protocolo mqtt)
    pub flags: u8,       //para subscribe siempre es 2(por protocolo mqtt)
    pub packet_identifier: u16,
    pub topic_filters: Vec<(String, u8)>, // (topic, qos)
}

impl SubscribeMessage {
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

        msg_bytes
    }
}