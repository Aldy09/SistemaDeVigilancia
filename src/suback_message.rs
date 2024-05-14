use std::{
    io::{Error, ErrorKind},
    mem::size_of,
};

use crate::subscribe_return_code::SubscribeReturnCode;

#[derive(Debug, PartialEq)]
pub struct SubAckMessage {
    message_type: u8, // Fixed header: 4 bits más sifgnificativos del primer byte, siempre 9
    reserved_flags: u8, // fixed header: 4 bits menos significativos del primer byte, 0
    packet_identifier: u16, // Variable header: 2 bytes
    return_codes: Vec<SubscribeReturnCode>, // Payload: 2 bytes cada uno, corresponde a cada topic_filter recibido.
}

impl SubAckMessage {
    pub fn new(packet_id: u16, return_codes: Vec<SubscribeReturnCode>) -> Self {
        Self {
            message_type: 9,
            reserved_flags: 0,
            packet_identifier: packet_id,
            return_codes,
        }
    }

    fn remaining_length(&self) -> u8 {
        // Calculo la rem_len
        let mut rem_len: u8 = 2; // 2 bytes de packet identifier
        for _return_code in &self.return_codes {
            rem_len += 2; // 2 bytes para enviar la longitud de cada return_code
        }
        rem_len
    }

    /// Pasa un struct SubAckMessage a bytes, usando big endian.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut msg_bytes = vec![];
        // Envío el primer byte, sus 4 bits superiores y 4 bits inferiores
        let mut byte_de_tipo = self.message_type << 4;
        byte_de_tipo |= self.reserved_flags;
        msg_bytes.extend(byte_de_tipo.to_be_bytes());

        // Calculo y envío la remaining length, un byte
        let rem_len = self.remaining_length();
        msg_bytes.extend(rem_len.to_be_bytes());

        // Variable header. Envío el packet identifier, 2 bytes
        msg_bytes.extend(self.packet_identifier.to_be_bytes());

        // Payload. Envío el vector de los returned_codes, elemento a elemento:
        for returned_code in &self.return_codes {
            msg_bytes.extend((*returned_code as u16).to_be_bytes());
        }

        msg_bytes
    }

    /// Recibe bytes, y los interpreta.
    /// Devuelve un struct SubAckMessage con los valores recibidos e interpretados.
    pub fn from_bytes(msg_bytes: Vec<u8>) -> Result<SubAckMessage, Error> {
        let size_of_u8 = size_of::<u8>();
        // Leo u8 byte de tipo y reserved flags
        let byte_de_tipo_y_flags = (&msg_bytes[0..size_of_u8])[0];
        let tipo = byte_de_tipo_y_flags >> 4;
        let reserved_flags = byte_de_tipo_y_flags & 0b0000_1111;

        // Leo u8 remaining length
        let rem_len = (&msg_bytes[size_of_u8..2 * size_of_u8])[0];
        let mut idx = 2 * size_of_u8;

        // Variable header. Leo u16 packet_id
        let size_of_u16 = size_of::<u16>();
        let packet_id = u16::from_be_bytes(
            msg_bytes[idx..idx + size_of_u16]
                .try_into()
                .map_err(|_| Error::new(ErrorKind::Other, "Error leyendo bytes subs msg."))?,
        ); // forma 1
        //let packet_id = u16::from_be_bytes([msg_bytes[idx], msg_bytes[idx+size_of_u8]]); // forma 2
        idx += size_of_u16;

        // Payload. Leo cada elemento del vector
        // Siendo que mqtt no envía la longitud del vector, utilizamos la remaining length
        let mut rem_len_leida: u8 = 2;
        let mut ret_codes: Vec<SubscribeReturnCode> = vec![];
        while rem_len_leida < rem_len {
            // Leo el u16
            let ret_code = u16::from_be_bytes([msg_bytes[idx], msg_bytes[idx + size_of_u8]]); // forma 2
            idx += size_of_u16;

            // Terminé de leer, agrego el elemento leído al vector de topics
            let elemento = SubscribeReturnCode::from_u16(ret_code)?;
            ret_codes.push(elemento);
            // Avanzo la rem_len_leida para saber cuándo termino de leer todos los elementos
            rem_len_leida += 2;
        }

        // Chequeo tipo correcto
        if tipo != 9 {
            return Err(Error::new(ErrorKind::Other, "Tipo incorrecto."));
        }

        let struct_interpretado = SubAckMessage {
            message_type: tipo,
            reserved_flags,
            packet_identifier: packet_id,
            return_codes: ret_codes,
        };

        Ok(struct_interpretado)
    }
}

#[cfg(test)]
mod test {
    use crate::{suback_message::SubAckMessage, subscribe_return_code::SubscribeReturnCode};

    #[test]
    fn test_1_suback_msg_se_crea_con_tipo_y_flag_adecuados() {
        let packet_id: u16 = 1;
        let return_codes = vec![SubscribeReturnCode::QoS1];
        let suback_msg = SubAckMessage::new(packet_id, return_codes);

        // Estos valores siempre son 9 y 0 respectivamente, para este tipo de mensaje
        assert_eq!(suback_msg.message_type, 9);
        assert_eq!(suback_msg.reserved_flags, 0);
    }

    #[test]
    fn test_2_suback_msg_se_pasa_a_bytes_y_se_interpreta_correctamente() {
        let packet_id: u16 = 1;
        let return_codes = vec![SubscribeReturnCode::QoS1];
        let suback_msg = SubAckMessage::new(packet_id, return_codes);

        let bytes_msg = suback_msg.to_bytes();

        let msg_reconstruido = SubAckMessage::from_bytes(bytes_msg);
        assert_eq!(msg_reconstruido.unwrap(), suback_msg);
    }

    #[test]
    fn test_3_suback_msg_se_pasa_a_bytes_e_interpreta_correctamente_con_varios_ret_codes() {
        let packet_id: u16 = 1;
        let mut return_codes = vec![SubscribeReturnCode::QoS1];
        return_codes.push(SubscribeReturnCode::QoS1); // agrego más topics al vector
        let suback_msg = SubAckMessage::new(packet_id, return_codes);

        let bytes_msg = suback_msg.to_bytes();

        let msg_reconstruido = SubAckMessage::from_bytes(bytes_msg);
        assert_eq!(msg_reconstruido.unwrap(), suback_msg);
    }
}
