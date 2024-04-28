use crate::subscribe_return_code::SubscribeReturnCode;

#[derive(Debug)]
pub struct SubackMessage {
    packet_identifier: u16,
    return_codes: Vec<SubscribeReturnCode>,
}