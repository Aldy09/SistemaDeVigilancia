use std::io::Write;

use crate::{
    connect_fixed_header::FixedHeader, connect_flags::ConnectFlags, connect_payload::Payload,
    connect_variable_header::VariableHeader,
};

#[derive(Debug)]
pub struct ConnectMessage<'a> {
    fixed_header: FixedHeader,
    variable_header: VariableHeader,
    payload: Payload<'a>,
}

impl<'a> ConnectMessage<'a> {
    pub fn new(
        client_id: &'a str,
        will_topic: Option<&'a str>,
        will_message: Option<&'a str>,
        username: Option<&'a str>,
        password: Option<&'a str>,
    ) -> Self {
        let fixed_header = FixedHeader {
            message_type: 1 << 4, // Siempre vale 1 (Podría llamarse message_type_byte, el tipo está en los 4 bits más signifs.)
            remaining_length: 0,  // Se actualizará más tarde
        };

        let variable_header = VariableHeader {
            protocol_name: [77, 81, 84, 84], // "MQTT" en ASCII
            protocol_level: 4,               // MQTT 3.1.1
            connect_flags: ConnectFlags {
                username_flag: username.is_some(),
                password_flag: password.is_some(),
                will_retain: false,
                will_qos: 0,
                will_flag: will_topic.is_some() && will_message.is_some(),
                clean_session: true,
                reserved: false,
            },
            // [] falta un keep alive, 2 bytes
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
        let variable_header_length = 5 + 1 + 1; // 5 bytes for "MQTT" (5=1+4), 1 byte for level, 1 byte for connect flags
        let length_string_u8 = 1;
        let payload_length = length_string_u8
            + self.payload.client_id.len()
            + length_string_u8
            + self.payload.will_topic.map_or(0, |s| s.len())
            + length_string_u8
            + self.payload.will_message.map_or(0, |s| s.len())
            + length_string_u8
            + self.payload.username.map_or(0, |s| s.len())
            + length_string_u8
            + self.payload.password.map_or(0, |s| s.len());
        
        // EL BUG!!! En el caso de los que pueden ser None, y en particular en los will_* que son los que usamos en none al usarlo,
        // se está sumando un length_string_u8 de todas formas!

        let quick_fix = 3; // Le resto 3, xq veo que los tres bytes que me faltan en el msj siguiente, los leyó de más acá
        // ToDo: revisar esta cuenta a ver de dónde son esos tres bytes que sobran. []
        // Listo un byte de una len. Ahora es 2 el desfasaje: los none de will_*?.

        (variable_header_length + payload_length) as u8
    }

    /// Pasa un ConnectMessage a bytes.
    pub fn to_bytes(&mut self) -> Vec<u8> {
        println!("DEBUG: TO BYTES {:?}", self);
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
        if let Some(will_topic) = self.payload.will_topic {
            bytes.push(will_topic.len() as u8);
            bytes.extend_from_slice(will_topic.as_bytes());
        }
        if let Some(will_message) = self.payload.will_message {
            bytes.push(will_message.len() as u8);
            bytes.extend_from_slice(will_message.as_bytes());
        }
        if let Some(username) = self.payload.username {
            bytes.push(username.len() as u8);
            bytes.extend_from_slice(username.as_bytes());
        }
        if let Some(password) = self.payload.password {
            bytes.push(password.len() as u8);
            bytes.extend_from_slice(password.as_bytes());
        }

        println!("DEBUG: TO BYTES {:?}", bytes);
        bytes
    }

    /// Parsea los bytes recibidos y devuelve un struct ConnectMessage.
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        println!("DEBUG: FROM BYTES {:?}", bytes);
        let fixed_header = FixedHeader {
            message_type: bytes[0],
            remaining_length: bytes[1],
        };

        let variable_header = VariableHeader {
            // el byte 2 es el protocol_name_len, debería valer siempre 4 que es la len de "MQTT".
            protocol_name: [bytes[3], bytes[4], bytes[5], bytes[6]],
            protocol_level: bytes[7],
            connect_flags: ConnectFlags::from_byte(bytes[8]),
        };

        // Indice donde comienza el payload (son 2 bytes de fixed header y 7 bytes de var header)
        let payload_start_index = 9;

        let variable_header_len: usize = 7; // (esto podría ser un método del variable header)
        // Calcular la longitud del payload
        // El payload length es rem_len - var header len ==> está bien, rem_len - 7 es correcto
        //let payload_length = fixed_header.remaining_length as usize - (5 + 1 + 1); // Total - 7 bytes de la cabecera fija y variable
        let payload_length = fixed_header.remaining_length as usize - variable_header_len; // Total - 7 bytes de la cabecera fija y variable
        println!("Leo rem len, leída de bytes: {}, payload len: {}", fixed_header.remaining_length, payload_length);

        // Extraer el payload del mensaje
        let payload_bytes = &bytes[payload_start_index..payload_start_index + payload_length];
        println!("Bytes del payload: {:?}", payload_bytes);

        // Procesar el payload según los flags y su longitud
        let payload = Self::process_payload(&variable_header.connect_flags, payload_bytes);

        // Verificar que el tipo sea correcto, siempre debe valer 1
        // algo del estilo if message_type != 1 {return error tipo incorrecto al crear ConnectMessage },
        // me va a cambiar la firma, lo dejo así ahora y dsp lo refactorizo []
        // Construir y retornar el mensaje ConnectMessage completo
        let msg = ConnectMessage {
            fixed_header,
            variable_header,
            payload,
        };
        println!("DEBUG: FROM BYTES {:?}", msg);
        msg
    }

    /// Parsea los bytes correspondientes al payload, a un struct payload con sus campos.
    pub fn process_payload(flags: &ConnectFlags, bytes_payload: &'a [u8]) -> Payload<'a> {
        let mut payload_start_index: usize = 0;

        // Extraer el client_id
        let client_id_length = bytes_payload[payload_start_index] as usize;
        let client_id = std::str::from_utf8(
            &bytes_payload[payload_start_index + 1..payload_start_index + 1 + client_id_length],
        )
        .unwrap();
        payload_start_index += 1 + client_id_length;

        // Extraer el will_topic y will_message si los flags lo indican
        let (will_topic, will_message) = if flags.will_flag {
            let will_topic_length = bytes_payload[payload_start_index] as usize;
            let will_topic = std::str::from_utf8(
                &bytes_payload
                    [payload_start_index + 1..payload_start_index + 1 + will_topic_length],
            )
            .unwrap();
            payload_start_index += 1 + will_topic_length;

            let will_message_length = bytes_payload[payload_start_index] as usize;
            let will_message = std::str::from_utf8(
                &bytes_payload
                    [payload_start_index + 1..payload_start_index + 1 + will_message_length],
            )
            .unwrap();
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
            .unwrap();
            payload_start_index += 1 + username_length;

            Some(username)
        } else {
            None
        };

        println!("hola");
        let _ = std::io::stdout().flush();
        // Extraer el password si los flags lo indican
        let password = if flags.password_flag {
            let password_length = bytes_payload[payload_start_index] as usize;
            let password = std::str::from_utf8(
                &bytes_payload[payload_start_index + 1..payload_start_index + 1 + password_length],
            )
            .unwrap();

            Some(password)
        } else {
            None
        };
        println!("no");
        

        Payload {
            client_id,
            will_topic,
            will_message,
            username,
            password,
        }
    }

    /// Devuelve el campo username del mensaje.
    pub fn get_user(&self) -> Option<&str> {
        self.payload.username
    }

    /// Devuelve el campo password del mensaje.
    pub fn get_passwd(&self) -> Option<&str> {
        self.payload.password
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_from_bytes_parsing_fixed_header() {
        // Creamos una instancia de ConnectMessage con algunos valores de ejemplo
        let mut connect_message = ConnectMessage::new(
            "test_client",
            Some("test/topic"),
            Some("test message"),
            Some("test_user"),
            Some("test_password"),
        );

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
        let mut connect_message = ConnectMessage::new(
            "test_client",
            Some("test/topic"),
            Some("test message"),
            Some("test_user"),
            Some("test_password"),
        );

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
        let mut connect_message = ConnectMessage::new(
            "test_client",
            Some("test/topic"),
            Some("test message"),
            Some("test_user"),
            Some("test_password"),
        );

        // Convertimos el mensaje a bytes
        let bytes = connect_message.to_bytes();

        // Convertimos los bytes a un nuevo mensaje
        let new_connect_message = ConnectMessage::from_bytes(&bytes);

        // Comprobamos que los mensajes son iguales
        //assert_eq!(connect_message.payload, new_connect_message.payload);
        assert_eq!(1, 2);
    }

    #[test]
    fn test_from_bytes_parsing_payload_get_user() {
        // Creamos una instancia de ConnectMessage con algunos valores de ejemplo
        let mut connect_message = ConnectMessage::new(
            "test_client",
            Some("test/topic"),
            Some("test message"),
            Some("test_user"),
            Some("test_password"),
        );

        // La función get_user obtiene el user del mensaje sin pasar a bytes
        assert_eq!(connect_message.get_user().unwrap(), "test_user");
        assert_eq!(connect_message.get_passwd().unwrap(), "test_password");

        // Convertimos el mensaje a bytes
        let bytes = connect_message.to_bytes();

        // Convertimos los bytes a un nuevo mensaje
        let new_connect_message = ConnectMessage::from_bytes(&bytes);

        // La función get_user obtiene el user del mensaje luego de convertirlo a mensaje desde bytes
        assert_eq!(new_connect_message.get_user(), connect_message.get_user());
        assert_eq!(
            new_connect_message.get_passwd(),
            connect_message.get_passwd()
        );
    }
}
