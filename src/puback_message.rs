use std::{io::{Error, ErrorKind}, mem::size_of};

#[derive(Debug, PartialEq)]
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
        
        // Variable header: packet_id y reason code
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

    pub fn msg_from_bytes(msg_bytes: Vec<u8>) -> Result<PubAckMessage, Error> {
        let size_of_u8 = size_of::<u8>();
        let mut idx = 0;
        // Leo byte de flags
        let flags_byte = (&msg_bytes[0..size_of_u8])[0];
        idx += size_of_u8;
        // Extraigo el tipo, del flags_byte
        let mut tipo: u8 = flags_byte & 0b1111_0000;
        tipo >>= 4;
        
        // Leo byte de remaining_len
        let remaining_len = (&msg_bytes[idx..idx+size_of_u8])[0];
        idx += size_of_u8;
        // Leo u16 de packet_id
        let size_of_u16 = size_of::<u16>();
        let packet_id = u16::from_be_bytes(msg_bytes[idx..idx+size_of_u16].try_into().map_err(|_| Error::new(ErrorKind::Other, "Error leyendo bytes puback msg."))?); // forma 1
        idx += size_of_u16;
        // Leo, si corresponde, u8 de reason code
        let mut puback_reason_code: u8 = 0;
        if remaining_len == 3 {
            puback_reason_code = (&msg_bytes[0..size_of_u8])[0];
        }
        
        Ok(PubAckMessage{ tipo, packet_id, puback_reason_code })
    }
    
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

    #[test]
    fn test_2_puback_msg_se_pasa_a_bytes_y_reconstruye_correctamente(){
        let msg = PubAckMessage::new(1,0);

        let msg_bytes = msg.to_bytes();

        let msg_reconstruido = PubAckMessage::msg_from_bytes(msg_bytes);

        assert_eq!(msg_reconstruido.unwrap(), msg);
    }
}