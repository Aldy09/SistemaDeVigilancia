
#[derive(Debug)]
pub struct PublishMessage<'a> {
    flags: u8,
    remaining_length: u8,
    topic_name: String,
    packet_identifier: u16,
    properties: Vec<(u8, u8)>, // [] ver si lo vamos a usar
    payload_to_publish: &'a[u8], // bytes
}

impl<'a> PublishMessage<'a> {
    pub fn to_bytes(&self) -> Vec<u8> {

        let mut msg_bytes: Vec<u8> = vec![];
        msg_bytes.extend(self.flags.to_be_bytes());
        

        msg_bytes
        
    }
}