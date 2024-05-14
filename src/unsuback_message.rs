pub struct Unsuback {
    fixed_header: FixedHeader,
    variable_header: VariableHeader,
    //No tiene payload
}

pub struct FixedHeader {

    //Message Type para UNSUBACK = 11 
    message_type: u8, //1er byte : 4bits
    reserved: u8, //1er byte : 4bits  seteados en 0

    //Remaining Length = variable_header.length = packet_identifier.length = 2
    remaining_length: u8, 
}

pub struct VariableHeader {
    packet_type_identifier_msb: u8, //1er byte 
    packet_type_identifier_lsb: u8, //2do byte
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

