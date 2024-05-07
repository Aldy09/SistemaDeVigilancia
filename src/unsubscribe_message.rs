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
    pub fn new(packet_identifier: u16, topics: Vec<String>){
        let variable_header = VariableHeader {
            packet_identifier,
        };

        let payload = Payload {
            topics,
        };

        let fixed_header = FixedHeader {
            message_type: 0b1010,
            reserved: 0010,
            remaining_length: 0,
        };

        let mut unsubscribe_message = UnsubscribeMessage {
            fixed_header,
            variable_header,
            payload,
        };

        unsubscribe_message.fixed_header.remaining_length = unsubscribe_message.calculate_remaining_length();
        unsubscribe_message
    }

    pub fn calculate_remaining_length(&self) -> usize {
        let packet_identifier_length = 2;
        let topics_length = self.payload.topics.iter().map(|topic| topic.len() + self.payload.topics.len).sum::<usize>();
        packet_identifier_length + topics_length;
    }

    pub fn to_bytes(&mut self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Fixed Header
        let combined = (self.fixed_header.message_type << 4) | self.fixed_header.reserved;
        bytes.push(combined);
        self.fixed_header.remaining_length = self.calculate_remaining_length();
        bytes.push(self.fixed_header.remaining_length);

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
            let topic = String::from_utf8(bytes[index + 1..index + 1 + topic_length].to_vec()).unwrap();
            topics.push(topic);
            index += 1 + topic_length;
        }

        Ok(UnsubscribeMessage {
            fixed_header: FixedHeader {
                message_type,
                reserved,
                remaining_length,
            },
            variable_header: VariableHeader {
                packet_identifier,
            },
            payload: Payload {
                topics,
            },
        })
    }
}
