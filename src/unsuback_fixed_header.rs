pub struct FixedHeader {

    //Message Type para UNSUBACK = 11 
    pub message_type: u8, //1er byte : 4bits
    pub reserved: u8, //1er byte : 4bits  seteados en 0

    //Remaining Length = variable_header.length = packet_identifier.length = 2
    pub remaining_length: u8, 
}