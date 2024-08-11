use crate::mqtt::messages::disconnect_fixed_header::FixedHeader;

#[derive(Debug, PartialEq)]
pub struct DisconnectMessage {
    fixed_header: FixedHeader,
}

impl DisconnectMessage {
    pub fn new() -> DisconnectMessage {
        let fixed_header = FixedHeader {
            message_type: 0b1110,
            reserved: 0b0000,
            remaining_length: 0,
        };

        DisconnectMessage { fixed_header }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        vec![self.fixed_header.message_type << 4 | self.fixed_header.reserved]
    }

    pub fn from_bytes(bytes: &[u8]) -> DisconnectMessage {
        let fixed_header = FixedHeader {
            message_type: bytes[0] >> 4,
            reserved: bytes[0] & 0b00001111,
            remaining_length: 0,
        };

        DisconnectMessage { fixed_header }
    }
}

impl Default for DisconnectMessage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::DisconnectMessage;

    #[test]
    fn test_disconnect_msg_to_and_from_bytes_works() {
        let original_msg = DisconnectMessage::new();
        let reconstructed_msg = DisconnectMessage::from_bytes(&original_msg.to_bytes());

        assert_eq!(reconstructed_msg, original_msg)
    }
}

// CHEQUEAR MAS ADELANTE
// 3.14.4 Response
// After sending a DISCONNECT Packet the Client:

// MUST close the Network Connection [MQTT-3.14.4-1].
// MUST NOT send any more Control Packets on that Network Connection [MQTT-3.14.4-2].

// On receipt of DISCONNECT the Server:

// MUST discard any Will Message associated with the current connection without publishing it, as described in Section 3.1.2.5 [MQTT-3.14.4-3].
// SHOULD close the Network Connection if the Client has not already done so.
