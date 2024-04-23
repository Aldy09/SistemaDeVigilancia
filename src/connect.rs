struct ConnectMessage {
    packet_type: u8,
    flags: u8,
    protocol_name: String,
    protocol_level: u8,
    connect_flags: u8,
    keep_alive: u16,
    client_id: String,
}