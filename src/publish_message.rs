use crate::publish_fixed_header::FixedHeader;
use crate::publish_flags::PublishFlags;
use crate::publish_payload::Payload;
use crate::publish_variable_header::VariableHeader;

#[derive(Debug, Clone)]
pub struct PublishMessage {
    fixed_header: FixedHeader,
    variable_header: VariableHeader,
    payload: Payload,
}

impl<'a> PublishMessage {
    pub fn new(
        message_type: u8,
        flags: PublishFlags,
        topic_name: &'a str,
        packet_identifier: Option<u16>,
        content: &'a [u8],
    ) -> Result<Self, String> {
        if !flags.is_qos_greater_than_0() && packet_identifier.is_some() {
            return Err("El packet_identifier debe ser None si qos = 0".to_string());
        }

        let variable_header = VariableHeader {
            topic_name: topic_name.to_string(),
            packet_identifier,
        };

        let payload = Payload {
            content: content.to_vec(),
        };

        let fixed_header = FixedHeader {
            message_type,
            flags,
            remaining_length: 0, //se actualizara mas adelante
        };

        let mut publish_message = PublishMessage {
            fixed_header,
            variable_header,
            payload,
        };

        publish_message.fixed_header.remaining_length =
            publish_message.calculate_remaining_length();

        Ok(publish_message)
    }

    fn calculate_remaining_length(&self) -> u8 {
        //remaining length = variable header + payload
        //variable header = topic_name + packet_identifier
        let rem_len_in_two_bytes = 2;
        let topic_name_length = self.variable_header.topic_name.len();
        let packet_identifier_length = match self.variable_header.packet_identifier {
            Some(_) => 2, //si qos > 0
            None => 0,    //si qos = 0
        };
        let payload_length = self.payload.content.len();

        (rem_len_in_two_bytes + topic_name_length + packet_identifier_length + payload_length) as u8
    }

    pub fn get_packet_identifier(&self) -> Option<u16> {
        self.variable_header.packet_identifier
    }

    ///Devuelve: Vector de bytes segun MQTT:
    /// 1er byte: meesage type y flags
    /// 2do byte: remaining_length
    /// 3er byte: topic_name_length_msb
    /// 4to byte: topic_name_length_lsb
    /// 5to byte: topic_name
    /// 6to byte: packet_identifier_msb opcional
    /// 7mo byte: packet_identifier_lsb opcional
    /// 8vo byte: payload
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Fixed Header
        let mut first_byte: u8 = self.fixed_header.message_type << 4; // message_type se coloca en los bits 4 a 7
        first_byte |= self.fixed_header.flags.to_flags_byte(); // flags se coloca en los bits 0 a 3
        bytes.push(first_byte);

        // Variable Header
        let topic_name_length = self.variable_header.topic_name.len() as u8;
        let remaining_length = 2
            + topic_name_length
            + 2 * self.variable_header.packet_identifier.is_some() as u8
            + self.payload.content.len() as u8;
        // Los 2 iniciales que le faltaban son de la longitud que se envía primero.
        // por ej si la string es "abc", primero se manda un 3 en dos bytes, y dsp "a", "b", "c".
        bytes.push(remaining_length);
        //bytes.push(topic_name_length);//longitud del topic_name en bytes (TENDRIA Q SER 2 BYTES msb y lsb)
        let topic_name_length_msb = ((topic_name_length as u16 >> 8) & 0xFF) as u8;
        let topic_name_length_lsb = topic_name_length;
        bytes.push(topic_name_length_msb);
        bytes.push(topic_name_length_lsb);
        bytes.extend_from_slice(self.variable_header.topic_name.as_bytes()); //extiendo el topic_name en la cantidad de bytes que tiene
        if let Some(packet_identifier) = self.variable_header.packet_identifier {
            bytes.push((packet_identifier >> 8) as u8); // MSB
            bytes.push(packet_identifier as u8); // LSB
        }

        // Payload
        bytes.extend_from_slice(&self.payload.content);

        bytes
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<PublishMessage, std::io::Error> {
        if bytes.len() < 5 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No hay suficientes bytes para un mensaje válido",
            )); // No hay suficientes bytes para un mensaje válido
        }

        // Fixed Header
        let first_byte = bytes[0];
        let message_type = first_byte >> 4; // message_type se extrae de los bits 4 a 7
        let flags = PublishFlags::from_flags_byte(first_byte & 0x0F)?; // flags se extrae de los bits 0 a 3
        let remaining_length = bytes[1];

        // Variable Header
        let topic_name_length = ((bytes[2] as usize) << 8) | (bytes[3] as usize);
        let topic_name = match String::from_utf8(bytes[4..4 + topic_name_length].to_vec()) {
            Ok(v) => v,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "El nombre del tema no es válido UTF-8",
                ))
            }
        };

        let mut packet_identifier = None;
        if remaining_length > (topic_name_length + 2) as u8 {
            packet_identifier = Some(
                ((bytes[4 + topic_name_length] as u16) << 8)
                    | (bytes[5 + topic_name_length] as u16),
            );
        }

        let payload_start = 4 + topic_name_length + 2 * packet_identifier.is_some() as usize;
        let payload_content = bytes[payload_start..].to_vec();

        Ok(Self {
            fixed_header: FixedHeader {
                message_type,
                flags,
                remaining_length,
            },
            variable_header: VariableHeader {
                topic_name,
                packet_identifier,
            },
            payload: Payload {
                content: payload_content,
            },
        })
    }

    pub fn get_topic(&self) -> String {
        self.variable_header.topic_name.to_string()
    }

    pub fn get_payload(&self) -> Vec<u8> {
        self.payload.content.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    ///Testea que si qos es 0, packet_identifier debe ser None.
    fn test_packet_identifier_none_if_qos_0() {
        let message = PublishMessage::new(
            1,                                   // message_type
            PublishFlags::new(0, 0, 0).unwrap(), // flags
            "test/topic",                        // topic_name
            Some(23),                            // packet_identifier
            &[1, 2, 3, 4, 5],                    // payload
        );

        assert!(message.is_err());
    }

    #[test]
    /// Testea que se pueda crear un mensaje Publish y pasarlo a bytes y luego reconstruirlo.
    fn test_publish_message_to_and_from_bytes() {
        let original_message = PublishMessage::new(
            1,                                   // message_type
            PublishFlags::new(0, 1, 0).unwrap(), // flags
            "test/topic",                        // topic_name
            Some(1234),                          // packet_identifier
            &[1, 2, 3, 4, 5],                    // payload
        )
        .unwrap();

        let bytes = original_message.to_bytes();
        let recovered_message = PublishMessage::from_bytes(bytes).unwrap();

        assert_eq!(
            original_message.fixed_header.message_type,
            recovered_message.fixed_header.message_type
        );
        assert_eq!(
            original_message.fixed_header.flags,
            recovered_message.fixed_header.flags
        );
        assert_eq!(
            original_message.fixed_header.remaining_length,
            recovered_message.fixed_header.remaining_length
        );
        assert_eq!(
            original_message.variable_header.topic_name,
            recovered_message.variable_header.topic_name
        );
        assert_eq!(
            original_message.variable_header.packet_identifier,
            recovered_message.variable_header.packet_identifier
        );
        assert_eq!(
            original_message.payload.content,
            recovered_message.payload.content
        );
    }
}
