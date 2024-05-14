use crate::{unsuback_fixed_header::FixedHeader, unsuback_variable_header::VariableHeader};

pub struct Unsuback {
    fixed_header: FixedHeader,
    variable_header: VariableHeader,
    //No tiene payload
}

impl Unsuback {
    pub fn new(packet_type_identifier_msb: u8, packet_type_identifier_lsb: u8) -> Unsuback {
        let variable_header = VariableHeader {
            packet_type_identifier_msb,
            packet_type_identifier_lsb,
        };

        let fixed_header = FixedHeader {
            message_type: 0b1011,
            reserved: 0b0000,
            remaining_length: 2,
        };

        Unsuback {
            fixed_header,
            variable_header,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        vec![
            self.fixed_header.message_type << 4 | self.fixed_header.reserved,
            self.fixed_header.remaining_length,
            self.variable_header.packet_type_identifier_msb,
            self.variable_header.packet_type_identifier_lsb,
        ]
    }

    pub fn from_bytes(bytes: &[u8]) -> Unsuback {
        let fixed_header = FixedHeader {
            message_type: bytes[0] >> 4,
            reserved: bytes[0] & 0b00001111,
            remaining_length: bytes[1],
        };

        let variable_header = VariableHeader {
            packet_type_identifier_msb: bytes[2],
            packet_type_identifier_lsb: bytes[3],
        };

        Unsuback {
            fixed_header,
            variable_header,
        }
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_bytes() {
        let unsuback = Unsuback::new(0x00, 0x01);
        let bytes = unsuback.to_bytes();

        assert_eq!(bytes, vec![0b1011_0000, 0x02, 0x00, 0x01]);
    }

    #[test]
    fn test_from_bytes() {
        let bytes = vec![0b1011_0000, 0x02, 0x00, 0x01];
        let unsuback = Unsuback::from_bytes(&bytes);

        assert_eq!(unsuback.fixed_header.message_type, 0b1011);
        assert_eq!(unsuback.fixed_header.reserved, 0b0000);
        assert_eq!(unsuback.fixed_header.remaining_length, 2);
        assert_eq!(unsuback.variable_header.packet_type_identifier_msb, 0x00);
        assert_eq!(unsuback.variable_header.packet_type_identifier_lsb, 0x01);
    }
}
