
#[derive(Debug)]
struct PubAckMessage {
    // rem len, calculable
    // Property len, pero string reason property es omitible.
    // Fixed header
    tipo: u8, // siempre vale 4; y son 4 bits al enviarlo, los restantes son ceros.
    // Variable header
    packet_id: u16,
    puback_reason_code: u8, // if al decodear, rem len = 2, es este campo = 0x00 éxito.
    // Si no usamos properties, la rem len debería ser siempre 2, xq el variable header es fijo en realidad.
    // El PubAck no lleva payload.
}

impl PubAckMessage {
    fn new(packet_id: u16, puback_reason_code: u8) -> Self {
        PubAckMessage { tipo: 4, packet_id, puback_reason_code }
    }

    pub fn to_bytes(&self) -> Vec<u8> {

        let mut msg_bytes: Vec<u8> = vec![];

        // Tipo
        let mut byte_de_flags: u8 = 0;
        byte_de_flags |= self.tipo << 4;
        msg_bytes.extend(byte_de_flags.to_be_bytes());

        // Remaining length
        let rem_len: u8 = self.remaining_length();
        msg_bytes.extend(rem_len.to_be_bytes());
        
        // Variable header: packet_id
        msg_bytes.extend(self.packet_id.to_be_bytes());
        if self.puback_reason_code != 0 {
            msg_bytes.extend(self.puback_reason_code.to_be_bytes());
        }        

        msg_bytes
    }

    /// Calcula la remaining length del puback message, que es variable porque puede
    /// o no enviarse un reason code. Es utilizada para pasaje del mensaje a y de bytes.
    fn remaining_length(&self) -> u8 {
        let mut rem_len: u8 = 0;
        rem_len += 2; // tam de u16 packet_id
        if self.puback_reason_code != 0 {
            rem_len += 1;
        } // Si es 0, significa success y no se envía, else sí se envía.
        rem_len
    }

    //msg_from_bytes
    
}

#[cfg(test)]
mod test{
    use super::PubAckMessage;

    #[test]
    fn test_1a_puback_msg_caso_success_tiene_rem_len_acorde(){
        let msg = PubAckMessage::new(1,0);
        // Con reason code 0, dicho campo no se envía, por lo que la rem len vale 2
        assert_eq!(msg.remaining_length(), 2);
    }
    #[test]
    fn test_1b_puback_msg_caso_error_tiene_rem_len_acorde(){
        let msg = PubAckMessage::new(1,8);
        // Con reason code no 0, dicho campo sí se envía, por lo que la rem len vale 3
        assert_eq!(msg.remaining_length(), 3);
    }

    /*#[test]
    fn test_2_puback_msg_aux_viendo_que_no_se_rompe(){
        let msg = PubAckMessage::new(1,0);

        msg.to_bytes(); // ok 
        //assert_eq!(msg.remaining_length(), 2);
    }*/
}