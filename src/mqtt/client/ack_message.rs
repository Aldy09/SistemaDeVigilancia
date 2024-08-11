//use std::fmt;

use crate::mqtt::messages::{puback_message::PubAckMessage, suback_message::SubAckMessage};

#[derive(Debug)]
pub enum ACKMessage {
    PubAck(PubAckMessage),
    SubAck(SubAckMessage),
}

impl ACKMessage {
    pub fn get_packet_id(&self) -> Option<u16> {
        match self {
            ACKMessage::PubAck(pub_ack_message) => Some(pub_ack_message.get_packet_id()),
            ACKMessage::SubAck(sub_ack_message) => Some(sub_ack_message.get_packet_id()),
        }
    }
}

// impl fmt::Debug for ACKMessage {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             ACKMessage::PubAck(msg) => write!(f, "PubAck({:?})", msg),
//             ACKMessage::SubAck(msg) => write!(f, "SubAck({:?})", msg),
//         }
//     }
// }
