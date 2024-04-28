use crate::subscribe_return_code::SubscribeReturnCode;

#[derive(Debug)]
#[allow(dead_code)]
pub struct SubAckMessage {
    packet_identifier: u16,
    return_codes: Vec<SubscribeReturnCode>,
}

