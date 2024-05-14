use crate::{unsuback_fixed_header::FixedHeader, unsuback_variable_header::VariableHeader};

pub struct Unsuback {
    fixed_header: FixedHeader,
    variable_header: VariableHeader,
    //No tiene payload
}

impl Unsuback {
    pub fn new(packet_type_identifier_msb: u8, packet_type_identifier_lsb: u8) -> Unsuback {
        let variable_header = VariableHeader { packet_type_identifier_msb, packet_type_identifier_lsb };

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
}

