use std::io::Error;

use crate::messages::{
    connack_fixed_header::FixedHeader, connack_session_present::SessionPresent,
    connack_variable_header::VariableHeader, connect_return_code::ConnectReturnCode,
};

#[derive(Debug)]
pub struct ConnackMessage {
    fixed_header: FixedHeader,
    variable_header: VariableHeader,
}

impl ConnackMessage {
    pub fn new(session_present: SessionPresent, return_code: ConnectReturnCode) -> Self {
        let fixed_header = FixedHeader {
            message_type: 0b0010_0000, // 0010 for MQTT Control Packet Type (2) and 0000 for reserved
            remaining_length: 2, // This is the length of the variable header. For the CONNACK Packet this has the value 2.
        };

        let connect_acknowledge_flags = match session_present {
            SessionPresent::PresentInLastSession => 0x01,
            SessionPresent::NotPresentInLastSession => 0x00,
        };

        let variable_header = VariableHeader {
            connect_acknowledge_flags,
            connect_return_code: return_code as u8,
        };

        ConnackMessage {
            fixed_header,
            variable_header,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // Fixed Header
        let message_type = self.fixed_header.message_type;
        let remaining_length = self.fixed_header.remaining_length;

        // Variable Header
        let connect_acknowledge_flags = self.variable_header.connect_acknowledge_flags;
        let connect_return_code = self.variable_header.connect_return_code;

        let bytes = vec![
            message_type,
            remaining_length,
            connect_acknowledge_flags,
            connect_return_code,
        ];

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let fixed_header = FixedHeader {
            message_type: bytes[0],
            remaining_length: bytes[1],
        };

        let variable_header = VariableHeader {
            connect_acknowledge_flags: bytes[2],
            connect_return_code: bytes[3],
        };

        // un if message_type != de (2<<4) {dar error}

        Ok(ConnackMessage {
            fixed_header,
            variable_header,
        })
    }
}

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let connack_packet = ConnackMessage::new(
            SessionPresent::PresentInLastSession,
            ConnectReturnCode::ConnectionAccepted,
        );
        assert_eq!(connack_packet.fixed_header.message_type, 0b0010_0000);
        assert_eq!(connack_packet.fixed_header.remaining_length, 2);
        assert_eq!(connack_packet.variable_header.connect_acknowledge_flags, 1);
        assert_eq!(connack_packet.variable_header.connect_return_code, 0);
    }

    #[test]
    fn test_to_bytes() {
        let connack_packet = ConnackMessage::new(
            SessionPresent::PresentInLastSession,
            ConnectReturnCode::ConnectionAccepted,
        );
        let bytes = connack_packet.to_bytes();
        assert_eq!(bytes, vec![0b0010_0000, 2, 1, 0]);
    }

    #[test]
    fn test_from_bytes() {
        let connack_packet = ConnackMessage::new(
            SessionPresent::PresentInLastSession,
            ConnectReturnCode::ConnectionAccepted,
        );
        let bytes = connack_packet.to_bytes();
        let connack_packet = ConnackMessage::from_bytes(&bytes).unwrap();
        assert_eq!(connack_packet.fixed_header.message_type, 0b0010_0000);
        assert_eq!(connack_packet.fixed_header.remaining_length, 2);
        assert_eq!(connack_packet.variable_header.connect_acknowledge_flags, 1);
        assert_eq!(connack_packet.variable_header.connect_return_code, 0);
    }
}
