use crate::mqtt::{messages::{
    connect_fixed_header::FixedHeader, connect_flags::ConnectFlags, connect_payload::Payload,
    connect_variable_header::VariableHeader,
}, mqtt_utils::will_message_utils::will_message::WillMessageData};

#[derive(Debug)]
pub struct ConnectMessage {
    fixed_header: FixedHeader,
    variable_header: VariableHeader,
    payload: Payload,
}

impl ConnectMessage {
    pub fn new(
        client_id: String,
        will_topic: Option<String>,
        will_message: Option<String>, //
        username: Option<String>,
        password: Option<String>,
        will_qos: u8,
    ) -> Self {
        let fixed_header = FixedHeader {
            message_type: 1 << 4,
            remaining_length: 0,
        };

        let variable_header = VariableHeader {
            protocol_name: [77, 81, 84, 84], // "MQTT" en ASCII
            protocol_level: 4,               // MQTT 3.1.1
            connect_flags: ConnectFlags {
                username_flag: username.is_some(),
                password_flag: password.is_some(),
                will_retain: true,
                will_qos,
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

        connect_message.fixed_header.remaining_length =
            connect_message.calculate_remaining_length();

        connect_message
    }

    fn calculate_remaining_length(&self) -> u8 {
        let variable_header_length = 5 + 1 + 1;
        let length_string_u8 = 1;
        let payload_length = length_string_u8
            + self.payload.client_id.len()
            + self
                .payload
                .will_topic
                .as_ref()
                .map_or(0, |s| s.len() + length_string_u8)
            + self
                .payload
                .will_message
                .as_ref()
                .map_or(0, |s| s.len() + length_string_u8)
            + self
                .payload
                .username
                .as_ref()
                .map_or(0, |s| s.len() + length_string_u8)
            + self
                .payload
                .password
                .as_ref()
                .map_or(0, |s| s.len() + length_string_u8);

        (variable_header_length + payload_length) as u8
    }

    /// Pasa un ConnectMessage a bytes.
    pub fn to_bytes(&mut self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Fixed Header
        bytes.push(self.fixed_header.message_type);
        self.fixed_header.remaining_length = self.calculate_remaining_length();
        bytes.push(self.fixed_header.remaining_length);

        // Variable Header
        let protocol_name_len: u8 = self.variable_header.protocol_name.len() as u8;
        bytes.push(protocol_name_len); // el valor 4, len de "MQTT".
        bytes.extend_from_slice(&self.variable_header.protocol_name);
        bytes.push(self.variable_header.protocol_level);
        let connect_flags = self.variable_header.connect_flags.to_byte();
        bytes.push(connect_flags);

        // Payload
        bytes.push(self.payload.client_id.len() as u8);
        bytes.extend_from_slice(self.payload.client_id.as_bytes());
        if let Some(will_topic) = self.payload.will_topic.clone() {
            bytes.push(will_topic.len() as u8);
            bytes.extend_from_slice(will_topic.as_bytes());
        }
        if let Some(will_message) = self.payload.will_message.clone() {
            bytes.push(will_message.len() as u8);
            bytes.extend_from_slice(will_message.as_bytes());
        }
        if let Some(username) = self.payload.username.clone() {
            bytes.push(username.len() as u8);
            bytes.extend_from_slice(username.as_bytes());
        }
        if let Some(password) = self.payload.password.clone() {
            bytes.push(password.len() as u8);
            bytes.extend_from_slice(password.as_bytes());
        }

        bytes
    }

    /// Parsea los bytes recibidos y devuelve un struct ConnectMessage.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let fixed_header = FixedHeader {
            message_type: bytes[0],
            remaining_length: bytes[1],
        };

        let variable_header = VariableHeader {
            // el byte 2 es el protocol_name_len, debería valer siempre 4 que es la len de "MQTT". []
            protocol_name: [bytes[3], bytes[4], bytes[5], bytes[6]],
            protocol_level: bytes[7],
            connect_flags: ConnectFlags::from_byte(bytes[8]),
        };

        // Indice donde comienza el payload (son 2 bytes de fixed header y 7 bytes de var header)
        let payload_start_index = 9;

        // Calcular la longitud del payload
        let variable_header_len: usize = 7; // (esto podría ser un método del variable header) // es payload_start_index - 2:
        let payload_length = fixed_header.remaining_length as usize - variable_header_len; // Total - 7 bytes del variable header
                                                                                           // Extraer el payload del mensaje
        let payload_bytes = &bytes[payload_start_index..payload_start_index + payload_length];

        // Procesar el payload según los flags y su longitud
        let payload = Self::process_payload(&variable_header.connect_flags, payload_bytes);

        // Verificar que el tipo sea correcto, siempre debe valer 1
        // algo del estilo if message_type != 1 {return error tipo incorrecto al crear ConnectMessage },
        // va a cambiar la firma, lo dejo así ahora y dsp lo refactorizo []
        // Construir y retornar el mensaje ConnectMessage completo
        ConnectMessage {
            fixed_header,
            variable_header,
            payload,
        }
    }

    /// Parsea los bytes correspondientes al payload, a un struct payload con sus campos.
    fn process_payload(flags: &ConnectFlags, bytes_payload: &[u8]) -> Payload {
        let mut payload_start_index: usize = 0;

        // Extraer el client_id
        let client_id_length = bytes_payload[payload_start_index] as usize;
        let client_id = std::str::from_utf8(
            &bytes_payload[payload_start_index + 1..payload_start_index + 1 + client_id_length],
        )
        .unwrap()
        .to_string(); // Convertir a String
        payload_start_index += 1 + client_id_length;

        // Extraer el will_topic y will_message si los flags lo indican
        let (will_topic, will_message) = if flags.will_flag {
            let will_topic_length = bytes_payload[payload_start_index] as usize;
            let will_topic = std::str::from_utf8(
                &bytes_payload
                    [payload_start_index + 1..payload_start_index + 1 + will_topic_length],
            )
            .unwrap()
            .to_string(); // Convertir a String
            payload_start_index += 1 + will_topic_length;

            let will_message_length = bytes_payload[payload_start_index] as usize;
            let will_message = std::str::from_utf8(
                &bytes_payload
                    [payload_start_index + 1..payload_start_index + 1 + will_message_length],
            )
            .unwrap()
            .to_string(); // Convertir a String
            payload_start_index += 1 + will_message_length;

            (Some(will_topic), Some(will_message))
        } else {
            (None, None)
        };

        // Extraer el username si los flags lo indican
        let username = if flags.username_flag {
            let username_length = bytes_payload[payload_start_index] as usize;
            let username = std::str::from_utf8(
                &bytes_payload[payload_start_index + 1..payload_start_index + 1 + username_length],
            )
            .unwrap()
            .to_string(); // Convertir a String
            payload_start_index += 1 + username_length;

            Some(username)
        } else {
            None
        };

        // Extraer el password si los flags lo indican
        let password = if flags.password_flag {
            let password_length = bytes_payload[payload_start_index] as usize;
            let password = std::str::from_utf8(
                &bytes_payload[payload_start_index + 1..payload_start_index + 1 + password_length],
            )
            .unwrap()
            .to_string(); // Convertir a String

            Some(password)
        } else {
            None
        };

        Payload {
            client_id,
            will_topic,
            will_message,
            username,
            password,
        }
    }

    /// Devuelve el campo username del mensaje.
    pub fn get_user(&self) -> Option<&String> {
        self.payload.username.as_ref()
    }

    /// Devuelve el campo password del mensaje.
    pub fn get_passwd(&self) -> Option<&String> {
        self.payload.password.as_ref()
    }

    /// Devuelve el campo client_id del mensaje.
    pub fn get_client_id(&self) -> Option<&String> {
        Some(&self.payload.client_id)
    }

    /// Devuelve un WillMessageAndTopic con los campos will_message y will_topic del mensaje
    /// si ambos son some, o None en caso contrario.
    pub fn get_will_to_publish(&self) -> Option<WillMessageData> {
        if let Some(msg) = &self.payload.will_message {
            if let Some(topic) = &self.payload.will_topic {

                let will_msg: WillMessageData = WillMessageData::new(
                    String::from(msg),
                    String::from(topic),
                    self.variable_header.connect_flags.will_qos,
                    self.variable_header.connect_flags.will_retain as u8,
                );
                return Some(will_msg);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn create_connect_message() -> ConnectMessage {
        ConnectMessage::new(
            "test_client".to_string(),
            Some("test/topic".to_string()),
            Some("test message".to_string()),
            Some("test_user".to_string()),
            Some("test_password".to_string()),
            0
        )
    }

    #[test]
    fn test_from_bytes_parsing_fixed_header() {
        // Creamos una instancia de ConnectMessage con algunos valores de ejemplo
        let mut connect_message = create_connect_message();

        // Convertimos el mensaje a bytes
        let bytes = connect_message.to_bytes();

        // Convertimos los bytes a un nuevo mensaje
        let new_connect_message = ConnectMessage::from_bytes(&bytes);

        // Comprobamos que los mensajes son iguales
        assert!(connect_message.fixed_header == new_connect_message.fixed_header);
    }

    #[test]
    fn test_from_bytes_parsing_variable_header() {
        // Creamos una instancia de ConnectMessage con algunos valores de ejemplo
        let mut connect_message = create_connect_message();

        let bytes = connect_message.to_bytes();

        // Convertimos los bytes a un nuevo mensaje
        let new_connect_message = ConnectMessage::from_bytes(&bytes);

        // Comprobamos que los mensajes son iguales
        assert_eq!(
            connect_message.variable_header,
            new_connect_message.variable_header
        );
    }

    #[test]
    fn test_from_bytes_parsing_payload() {
        // Creamos una instancia de ConnectMessage con algunos valores de ejemplo
        let mut connect_message = create_connect_message();

        // Convertimos el mensaje a bytes
        let bytes = connect_message.to_bytes();

        // Convertimos los bytes a un nuevo mensaje
        let new_connect_message = ConnectMessage::from_bytes(&bytes);

        // Comprobamos que los mensajes son iguales
        assert_eq!(connect_message.payload, new_connect_message.payload);
    }

    #[test]
    fn test_from_bytes_parsing_payload_get_user_get_passwd() {
        // Creamos una instancia de ConnectMessage con algunos valores de ejemplo
        let mut connect_message = create_connect_message();

        // La función get_user obtiene el user del mensaje sin pasar a bytes
        assert_eq!(
            *connect_message.get_user().unwrap(),
            "test_user".to_string()
        );
        assert_eq!(
            *connect_message.get_passwd().unwrap(),
            "test_password".to_string()
        );

        // Convertimos el mensaje a bytes
        let bytes = connect_message.to_bytes();

        // Convertimos los bytes a un nuevo mensaje
        let new_connect_message = ConnectMessage::from_bytes(&bytes);

        // La función get_user obtiene el user del mensaje luego de convertirlo a mensaje desde bytes
        assert_eq!(new_connect_message.get_user().unwrap(), "test_user");
        assert_eq!(new_connect_message.get_passwd().unwrap(), "test_password");
    }

    #[test]
    fn test_from_bytes_works_properly_with_none_fields() {
        // Creamos una instancia de ConnectMessage con algunos valores en None
        let mut connect_message = ConnectMessage::new(
            "test_client".to_string(),
            None,
            None,
            Some("test_user".to_string()),
            Some("test_password123".to_string()),
            0
        );
        // Convertimos el mensaje a bytes
        let bytes = connect_message.to_bytes();

        // Convertimos los bytes a un nuevo mensaje
        let new_connect_message = ConnectMessage::from_bytes(&bytes);

        // Comprobamos que los mensajes son iguales
        assert_eq!(connect_message.payload, new_connect_message.payload);
    }
}
