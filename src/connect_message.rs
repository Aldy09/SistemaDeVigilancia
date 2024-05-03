use crate::{connect_fixed_header::FixedHeader, connect_flags::ConnectFlags, connect_message_payload::Payload, connect_variable_header::VariableHeader};


#[derive(Debug)]
pub struct ConnectMessage<'a> {
    fixed_header: FixedHeader,
    variable_header: VariableHeader,
    payload: Payload<'a>,
}

impl <'a> ConnectMessage<'a> {
    pub fn new(
        message_type: u8,
        client_id: &'a str,
        will_topic: Option<&'a str>,
        will_message: Option<&'a str>,
        username: Option<&'a str>,
        password: Option<&'a str>,
    ) -> Self {
        let fixed_header = FixedHeader {
            message_type,
            remaining_length: 0, // Se actualizará más tarde
        };

        let variable_header = VariableHeader {
            protocol_name: [77, 81, 84, 84], // "MQTT" en ASCII
            protocol_level: 4, // MQTT 3.1.1
            connect_flags: ConnectFlags {
                username_flag: username.is_some(),
                password_flag: password.is_some(),
                will_retain: false,
                will_qos: 0,
                will_flag: will_topic.is_some() && will_message.is_some(),
                clean_session: true,
                reserved: false,
            },
        };

        let payload = Payload {
            client_id,
            will_topic,
            will_message,
            username,
            password,
        };

        let mut connect_message = ConnectMessage {
            fixed_header,
            variable_header,
            payload,
        };

        connect_message.fixed_header.remaining_length = connect_message.calculate_remaining_length();

        connect_message
    }
    
    fn calculate_remaining_length(&self) -> u8 {
        let variable_header_length = 5 + 1 + 1; // 5 bytes for "MQTT", 1 byte for level, 1 byte for connect flags
        let payload_length = self.payload.client_id.len()
            + self.payload.will_topic.map_or(0, |s| s.len())
            + self.payload.will_message.map_or(0, |s| s.len())
            + self.payload.username.map_or(0, |s| s.len())
            + self.payload.password.map_or(0, |s| s.len());

        (variable_header_length + payload_length) as u8
    }

    pub fn to_bytes(&mut self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Fixed Header
        bytes.push(self.fixed_header.message_type);
        self.fixed_header.remaining_length = self.calculate_remaining_length();
        bytes.push(self.fixed_header.remaining_length);

        // Variable Header
        bytes.extend_from_slice(&self.variable_header.protocol_name);
        bytes.push(self.variable_header.protocol_level);
        let connect_flags = self.variable_header.connect_flags.to_byte();
        bytes.push(connect_flags);

        // Payload
        bytes.extend_from_slice(self.payload.client_id.as_bytes());
        if let Some(will_topic) = self.payload.will_topic {
            bytes.extend_from_slice(will_topic.as_bytes());
        }
        if let Some(will_message) = self.payload.will_message {
            bytes.extend_from_slice(will_message.as_bytes());
        }
        if let Some(username) = self.payload.username {
            bytes.extend_from_slice(username.as_bytes());
        }
        if let Some(password) = self.payload.password {
            bytes.extend_from_slice(password.as_bytes());
        }

        bytes
    }
}
