// UNSUBSCRIBE MESSAGE
#[derive(Debug)]
pub struct UnsubscribeMessage {
    fixed_header: FixedHeader,
    variable_header: VariableHeader,
    payload: Payload,
}

#[derive(Debug)]
pub struct FixedHeader {
    message_type: u8,
    reserved: u8,
    remaining_length: usize,
}

#[derive(Debug)]
pub struct VariableHeader {
    packet_identifier: u16,
}

#[derive(Debug)]
pub struct Payload {
    topics: Vec<String>,
}

impl UnsubscribeMessage {
    
    // Crea un nuevo mensaje UNSUBSCRIBE
    pub fn new(packet_identifier: u16, topics: Vec<String>) -> UnsubscribeMessage {
        let variable_header = VariableHeader { packet_identifier };

        let payload = Payload { topics };

        let fixed_header = FixedHeader {
            message_type: 0b1010,
            reserved: 0b0010,
            remaining_length: 0,
        };

        let mut unsubscribe_message = UnsubscribeMessage {
            fixed_header,
            variable_header,
            payload,
        };

        unsubscribe_message.fixed_header.remaining_length =
            unsubscribe_message.calculate_remaining_length();
        unsubscribe_message
    }

    // Calcula el tamaño del remaining = variable header + payload
    pub fn calculate_remaining_length(&self) -> usize {
        let packet_identifier_length = 2;
        //let topics_length = self.payload.topics.iter().map(|topic| topic.len() + self.payload.topics.len()).sum::<usize>();
        let topics_length = self
            .payload
            .topics
            .iter()
            .map(|topic| 1 + topic.len())
            .sum::<usize>();
        packet_identifier_length + topics_length
    }

    pub fn to_bytes(&mut self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Fixed Header
        let combined = (self.fixed_header.message_type << 4) | self.fixed_header.reserved;
        bytes.push(combined);
        self.fixed_header.remaining_length = self.calculate_remaining_length();
        bytes.push(self.fixed_header.remaining_length as u8);

        // Variable Header
        bytes.push((self.variable_header.packet_identifier >> 8) as u8); // MSB
        bytes.push((self.variable_header.packet_identifier & 0xFF) as u8); // LSB

        // Payload
        for topic in &self.payload.topics {
            bytes.push(topic.len() as u8); //Obviamos el MSB, y solo trabajamos con los LSB
            bytes.extend_from_slice(topic.as_bytes());
        }

        bytes
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<UnsubscribeMessage, std::io::Error> {
        if bytes.len() < 4 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No hay suficientes bytes para un mensaje válido",
            )); // No hay suficientes bytes para un mensaje válido
        }

        // Fixed Header
        let first_byte = bytes[0];
        let message_type = first_byte >> 4; // message_type se extrae de los bits 4 a 7
        let reserved = first_byte & 0x0F; // reserved se extrae de los bits 0 a 3
        let remaining_length = bytes[1];

        // Variable Header
        let packet_identifier = ((bytes[2] as u16) << 8) | (bytes[3] as u16);

        // Payload
        let mut topics = Vec::new();
        let mut index = 4;
        while index < bytes.len() {
            let topic_length = bytes[index] as usize;
            let topic =
                String::from_utf8(bytes[index + 1..index + 1 + topic_length].to_vec()).unwrap();
            topics.push(topic);
            index += 1 + topic_length;
        }

        Ok(UnsubscribeMessage {
            fixed_header: FixedHeader {
                message_type,
                reserved,
                remaining_length: remaining_length as usize,
            },
            variable_header: VariableHeader { packet_identifier },
            payload: Payload { topics },
        })
    }
}



#[cfg(test)]
mod test {

    use super::*;

    //Testea que si no hay suficientes bytes para un mensaje válido, retorne un error
    #[test]
    fn test_unsubscribe_message_from_bytes_error() {
        let bytes = vec![0b1010_0010];
        let unsubscribe_message = UnsubscribeMessage::from_bytes(bytes);
        assert!(unsubscribe_message.is_err());
    }

    //Testea que el mensaje se pueda convertir a bytes
    #[test]
    fn test_unsubscribe_message_to_bytes() {
        let packet_identifier = 10;
        let topics = vec!["topic1".to_string(), "topic2".to_string()];
        let mut unsubscribe_message = UnsubscribeMessage::new(packet_identifier, topics);
        let bytes = unsubscribe_message.to_bytes();
        let expected_bytes = vec![
            0b1010_0010, // Fixed Header 10 y 2 de reserved
            0x10, // Remaining Length 16 = 2(packet_identifier) + ((1 + 6) + (1 + 6)):topic1 y topic2
            0x00,
            0x0A, // Packet Identifier
            0x06,
            0x74,
            0x6F,
            0x70,
            0x69,
            0x63,
            0x31, // Topic1 0x06 es el largo de la palabra,0x74 es la t, 0x6F es la o, 0x70 es la p, 0x69 es la i, 0x63 es la c, 0x31 es el 1
            0x06,
            0x74,
            0x6F,
            0x70,
            0x69,
            0x63,
            0x32, // Topic2
        ];
        assert_eq!(bytes, expected_bytes);
    }

    //Testea que el mensaje se pueda convertir de bytes a mensaje
    #[test]
    fn test_unsubscribe_message_from_bytes() {
        let bytes = vec![
            0b1010_0010, // Fixed Header
            0x10,        // Remaining Length
            0x00,
            0x0B, // Packet Identifier
            0x06, //Topic 1
            0x74,
            0x6F,
            0x70,
            0x69,
            0x63,
            0x31, 
            0x06, // Topic2
            0x74,
            0x6F,
            0x70,
            0x69,
            0x63,
            0x32, 
        ];
        let unsubscribe_message = UnsubscribeMessage::from_bytes(bytes).unwrap();
        assert_eq!(unsubscribe_message.fixed_header.message_type, 0b1010);
        assert_eq!(unsubscribe_message.fixed_header.reserved, 0b0010);
        assert_eq!(unsubscribe_message.fixed_header.remaining_length, 0x10);
        assert_eq!(unsubscribe_message.variable_header.packet_identifier, 11);
        assert_eq!(
            unsubscribe_message.payload.topics,
            vec!["topic1".to_string(), "topic2".to_string()]
        );
    }

    //testea de que el mensaje se pueda convertir a bytes y de bytes a mensaje
    #[test]
    fn test_unsubscribe_message_to_bytes_and_back() {
        let packet_identifier = 12;
        let topics = vec!["topic1".to_string(), "topic2".to_string()];
        let mut unsubscribe_message = UnsubscribeMessage::new(packet_identifier, topics);

        let bytes = unsubscribe_message.to_bytes();

        let new_unsubscribe_message = UnsubscribeMessage::from_bytes(bytes).unwrap();
        assert_eq!(
            unsubscribe_message.fixed_header.message_type,
            new_unsubscribe_message.fixed_header.message_type
        );
        assert_eq!(
            unsubscribe_message.fixed_header.reserved,
            new_unsubscribe_message.fixed_header.reserved
        );
        assert_eq!(
            unsubscribe_message.fixed_header.remaining_length,
            new_unsubscribe_message.fixed_header.remaining_length
        );
        assert_eq!(
            unsubscribe_message.variable_header.packet_identifier,
            new_unsubscribe_message.variable_header.packet_identifier
        );
        assert_eq!(
            unsubscribe_message.payload.topics,
            new_unsubscribe_message.payload.topics
        );
    }
}
