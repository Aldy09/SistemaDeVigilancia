#[derive(Debug)]
struct SubackMessage {
    packet_identifier: u16,
    return_codes: Vec<SubscribeReturnCode>,
}