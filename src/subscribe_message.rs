#[allow(dead_code)]
pub struct SubscribeMessage {
    packet_type: u8,//para subscribe siempre es 8(por protocolo mqtt)
    flags: u8, //para subscribe siempre es 2(por protocolo mqtt) 
    packet_identifier: u16, 
    topic_filters: Vec<(String,u8)>, // (topic, qos)
}
