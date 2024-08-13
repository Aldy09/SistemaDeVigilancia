extern crate block_modes;
extern crate des;
extern crate hex;

use std::io::{Error, ErrorKind};
use std::time::{SystemTime, UNIX_EPOCH};

// use des::cipher::generic_array::GenericArray;
// use des::cipher::NewBlockCipher;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use des::TdesEde3;

// Tipo para el modo CBC con padding PKCS7
type TdesEde3Cbc = Cbc<TdesEde3, Pkcs7>;

// Clave y vector de inicialización de ejemplo (debe ser de 24 y 8 bytes respectivamente)
const KEY: [u8; 24] = [0x01; 24]; // Esto es solo un ejemplo, usa claves seguras en producción
const IV: [u8; 8] = [0x02; 8];

use crate::mqtt::messages::publish_fixed_header::FixedHeader;
use crate::mqtt::messages::publish_flags::PublishFlags;
use crate::mqtt::messages::publish_payload::Payload;
use crate::mqtt::messages::publish_variable_header::VariableHeader;

type TimestampType = u128;
const  TIMESTAMP_LENGHT: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct PublishMessage {
    fixed_header: FixedHeader,
    variable_header: VariableHeader,
    payload: Payload,
    timestamp: TimestampType,
}

impl<'a> PublishMessage {
    // pub fn new(
    //     flags: PublishFlags,
    //     topic_name: &'a str,
    //     packet_identifier: Option<u16>,
    //     content: &'a [u8],
    // ) -> Result<Self, Error> {
    //     if !flags.is_qos_greater_than_0() && packet_identifier.is_some() {
    //         return Err(Error::new(
    //             ErrorKind::InvalidData,
    //             "El packet_identifier debe ser None si qos = 0".to_string(),
    //         ));
    //     }

    //     let variable_header = VariableHeader {
    //         topic_name: topic_name.to_string(),
    //         packet_identifier,
    //     };

    //     //Hacer que la variable content se encripte por un algortimo de encriptacion SHA256
    //     let content = encrypt_3des(content);

    //     let payload = Payload {
    //         content: content.to_vec(),
    //     };

    //     //aux: let payload = Payload { content: content.to_vec() };

    //     let fixed_header = FixedHeader {
    //         flags,
    //         remaining_length: 0, //se actualizara mas adelante
    //     };

    //     let mut publish_message = PublishMessage {
    //         fixed_header,
    //         variable_header,
    //         payload,
    //     };

    //     publish_message.fixed_header.remaining_length =
    //         publish_message.calculate_remaining_length();

    //     Ok(publish_message)
    // }

    // fn calculate_remaining_length(&self) -> u8 {
    //     //aux: remaining length = variable header + payload
    //     //aux: variable header = topic_name + packet_identifier
    //     let rem_len_in_two_bytes = 2;
    //     let topic_name_length = self.variable_header.topic_name.len();
    //     let packet_identifier_length = match self.variable_header.packet_identifier {
    //         Some(_) => 2, //si qos > 0
    //         None => 0,    //si qos = 0
    //     };
    //     let payload_length = self.payload.content.len();

    //     (rem_len_in_two_bytes + topic_name_length + packet_identifier_length + payload_length) as u8
    // }

    pub fn new(
        flags: PublishFlags,
        topic_name: &'a str,
        packet_identifier: Option<u16>,
        content: &'a [u8],
    ) -> Result<Self, Error> {
        if !flags.is_qos_greater_than_0() && packet_identifier.is_some() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "El packet_identifier debe ser None si qos = 0".to_string(),
            ));
        }

        let variable_header = VariableHeader {
            topic_name: topic_name.to_string(),
            packet_identifier,
        };

        let content = encrypt_3des(content);

        let payload = Payload {
            content: content.to_vec(),
        };

        let fixed_header = FixedHeader {
            flags,
            remaining_length: 0, // se actualizará más adelante
        };

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let mut publish_message = PublishMessage {
            fixed_header,
            variable_header,
            payload,
            timestamp,
        };

        publish_message.fixed_header.remaining_length =
            publish_message.calculate_remaining_length_2();

        Ok(publish_message)
    }

    fn calculate_remaining_length_2(&self) -> u8 {
        //aux: remaining length = variable header + payload
        //aux: variable header = topic_name + packet_identifier
        let rem_len_in_two_bytes = 2;
        let topic_name_length = self.variable_header.topic_name.len();
        let packet_identifier_length = match self.variable_header.packet_identifier {
            Some(_) => 2, //si qos > 0
            None => 0,    //si qos = 0
        };
        let payload_length = self.payload.content.len();
        let timestamp_length = TIMESTAMP_LENGHT; // tamaño de u128

        (rem_len_in_two_bytes
            + topic_name_length
            + packet_identifier_length
            + payload_length
            + timestamp_length) as u8
    }

    pub fn get_packet_id(&self) -> Option<u16> {
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
    // pub fn to_bytes(&self) -> Vec<u8> {
    //     let mut bytes = Vec::new();

    //     // Fixed Header
    //     let first_byte = self.fixed_header.flags.to_flags_byte(); // flags contiene los flags y tmb incluye el tipo.
    //     bytes.push(first_byte);

    //     // Variable Header
    //     let topic_name_length = self.variable_header.topic_name.len() as u8;
    //     let remaining_length = 2
    //         + topic_name_length
    //         + 2 * self.variable_header.packet_identifier.is_some() as u8
    //         + self.payload.content.len() as u8;
    //     // Los 2 iniciales que le faltaban son de la longitud que se envía primero.
    //     // por ej si la string es "abc", primero se manda un 3 en dos bytes, y dsp "a", "b", "c".
    //     bytes.push(remaining_length);
    //     //bytes.push(topic_name_length);//longitud del topic_name en bytes (TENDRIA Q SER 2 BYTES msb y lsb)
    //     let topic_name_length_msb = ((topic_name_length as u16 >> 8) & 0xFF) as u8;
    //     let topic_name_length_lsb = topic_name_length;
    //     bytes.push(topic_name_length_msb);
    //     bytes.push(topic_name_length_lsb);
    //     bytes.extend_from_slice(self.variable_header.topic_name.as_bytes()); //extiendo el topic_name en la cantidad de bytes que tiene
    //     if let Some(packet_identifier) = self.variable_header.packet_identifier {
    //         bytes.push((packet_identifier >> 8) as u8); // MSB
    //         bytes.push(packet_identifier as u8); // LSB
    //     }

    //     // Payload
    //     bytes.extend_from_slice(&self.payload.content);

    //     bytes
    // }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let first_byte = self.fixed_header.flags.to_flags_byte();
        bytes.push(first_byte);

        let topic_name_length = self.variable_header.topic_name.len() as u8;
        let remaining_length = 2
            + topic_name_length
            + 2 * self.variable_header.packet_identifier.is_some() as u8
            + self.payload.content.len() as u8
            + TIMESTAMP_LENGHT as u8; // tamaño del timestamp
        bytes.push(remaining_length);

        let topic_name_length_msb = ((topic_name_length as u16 >> 8) & 0xFF) as u8;
        let topic_name_length_lsb = topic_name_length;
        bytes.push(topic_name_length_msb);
        bytes.push(topic_name_length_lsb);
        bytes.extend_from_slice(self.variable_header.topic_name.as_bytes());
        if let Some(packet_identifier) = self.variable_header.packet_identifier {
            bytes.push((packet_identifier >> 8) as u8);
            bytes.push(packet_identifier as u8);
        }

        bytes.extend_from_slice(&self.payload.content);

        let timestamp_bytes = self.timestamp.to_be_bytes();
        bytes.extend_from_slice(&timestamp_bytes);

        bytes
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<PublishMessage, std::io::Error> {
        if bytes.len() < 13 {
            // Mínimo 5 bytes + 8 bytes de timestamp
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No hay suficientes bytes para un mensaje válido",
            ));
        }

        let first_byte = bytes[0];
        let flags = PublishFlags::from_flags_byte(first_byte)?;
        let remaining_length = bytes[1];

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
        let payload_end = bytes.len() - TIMESTAMP_LENGHT;
        let payload_content = bytes[payload_start..payload_end].to_vec();

        // Cambiar el u128 en caso de que se cambie el tipo de dato del TIMESTAMP
        let timestamp = u128::from_be_bytes(bytes[payload_end..].try_into().unwrap());

        Ok(Self {
            fixed_header: FixedHeader {
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
            timestamp,
        })
    }

    // pub fn from_bytes(bytes: Vec<u8>) -> Result<PublishMessage, std::io::Error> {
    //     if bytes.len() < 5 {
    //         return Err(std::io::Error::new(
    //             std::io::ErrorKind::InvalidData,
    //             "No hay suficientes bytes para un mensaje válido",
    //         )); // No hay suficientes bytes para un mensaje válido
    //     }

    //     // Fixed Header
    //     let first_byte = bytes[0];
    //     let flags = PublishFlags::from_flags_byte(first_byte)?; // flags se extrae de los bits 0 a 3
    //     let remaining_length = bytes[1];

    //     // Variable Header
    //     let topic_name_length = ((bytes[2] as usize) << 8) | (bytes[3] as usize);
    //     let topic_name = match String::from_utf8(bytes[4..4 + topic_name_length].to_vec()) {
    //         Ok(v) => v,
    //         Err(_) => {
    //             return Err(std::io::Error::new(
    //                 std::io::ErrorKind::InvalidData,
    //                 "El nombre del tema no es válido UTF-8",
    //             ))
    //         }
    //     };

    //     let mut packet_identifier = None;
    //     if remaining_length > (topic_name_length + 2) as u8 {
    //         packet_identifier = Some(
    //             ((bytes[4 + topic_name_length] as u16) << 8)
    //                 | (bytes[5 + topic_name_length] as u16),
    //         );
    //     }

    //     let payload_start = 4 + topic_name_length + 2 * packet_identifier.is_some() as usize;
    //     let payload_content = bytes[payload_start..].to_vec();

    //     Ok(Self {
    //         fixed_header: FixedHeader {
    //             flags, // Incluye el msg_type.
    //             remaining_length,
    //         },
    //         variable_header: VariableHeader {
    //             topic_name,
    //             packet_identifier,
    //         },
    //         payload: Payload {
    //             content: payload_content,
    //         },
    //     })
    // }

    pub fn get_topic(&self) -> String {
        self.variable_header.topic_name.to_string()
    }

    pub fn get_payload(&self) -> Vec<u8> {
        decrypt_3des(&self.payload.content)
        //aux: self.payload.content.to_vec()
    }

    pub fn get_qos(&self) -> u8 {
        self.fixed_header.flags.get_qos()
    }

    pub fn get_topic_name(&self) -> String {
        self.variable_header.topic_name.to_string()
    }

    pub fn get_timestamp(&self) -> TimestampType {
        self.timestamp
    }
}

use super::packet_type::PacketType;
use crate::mqtt::messages::message::Message;
//Trait Message
impl Message for PublishMessage {
    fn get_packet_id(&self) -> Option<u16> {
        self.get_packet_id()
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn get_type(&self) -> PacketType {
        PacketType::Publish
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn encrypt_3des(data: &[u8]) -> Vec<u8> {
    let cipher = TdesEde3Cbc::new_from_slices(&KEY, &IV).unwrap();
    cipher.encrypt_vec(data)
}

fn decrypt_3des(encrypted_data: &[u8]) -> Vec<u8> {
    let cipher = TdesEde3Cbc::new_from_slices(&KEY, &IV).unwrap();
    cipher.decrypt_vec(encrypted_data).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_publish_message() -> Result<PublishMessage, Error> {
        let flags = PublishFlags::new(0, 1, 0).unwrap();
        let topic_name = "test/topic";
        let packet_identifier = Some(42);
        let content = b"Hello, world!";

        PublishMessage::new(flags, topic_name, packet_identifier, content)
    }

    #[test]
    fn test_to_bytes() {
        let publish_message = create_test_publish_message().unwrap();
        let bytes = publish_message.to_bytes();

        // Deserializa los bytes nuevamente a un PublishMessage
        let deserialized_message = PublishMessage::from_bytes(bytes).unwrap();

        // Verifica que los campos importantes sean iguales
        assert_eq!(
            publish_message.fixed_header.flags,
            deserialized_message.fixed_header.flags
        );
        assert_eq!(
            publish_message.variable_header.topic_name,
            deserialized_message.variable_header.topic_name
        );
        assert_eq!(
            publish_message.variable_header.packet_identifier,
            deserialized_message.variable_header.packet_identifier
        );
        assert_eq!(
            publish_message.payload.content,
            deserialized_message.payload.content
        );
        assert_eq!(publish_message.timestamp, deserialized_message.timestamp);
    }

    #[test]
    fn test_bytes_and_comparison() {
        let publish_message = create_test_publish_message().unwrap();
        let bytes = publish_message.to_bytes();

        // Deserializa los bytes nuevamente a un PublishMessage
        let deserialized_message = PublishMessage::from_bytes(bytes).unwrap();

        assert_eq!(
            publish_message.get_timestamp(),
            deserialized_message.get_timestamp()
        );
    }

    #[test]
    fn test_timestamp_comparison() {
        let msg1 = create_test_publish_message().unwrap();
        // Simular un retraso
        std::thread::sleep(std::time::Duration::from_secs(2));
        let msg2 = create_test_publish_message().unwrap();

        // msg2 debería tener un timestamp más reciente que msg1
        assert!(msg1.get_timestamp() < msg2.get_timestamp());
    }

    // #[test]
    // ///Testea que si qos es 0, packet_identifier debe ser None.
    // fn test_packet_identifier_none_if_qos_0() {
    //     let message = PublishMessage::new(
    //         PublishFlags::new(0, 0, 0).unwrap(), // flags, se crea con msg_type=3.
    //         "test/topic",                        // topic_name
    //         Some(23),                            // packet_identifier
    //         &[1, 2, 3, 4, 5],                    // payload
    //     );

    //     assert!(message.is_err());
    // }

    // #[test]
    // /// Testea que se pueda crear un mensaje Publish y pasarlo a bytes y luego reconstruirlo.
    // fn test_publish_message_to_and_from_bytes() {
    //     let original_message = PublishMessage::new(
    //         PublishFlags::new(0, 1, 0).unwrap(), // flags
    //         "test/topic",                        // topic_name
    //         Some(1234),                          // packet_identifier
    //         &[1, 2, 3, 4, 5],                    // payload
    //     )
    //     .unwrap();

    //     let bytes = original_message.to_bytes();
    //     let recovered_message = PublishMessage::from_bytes(bytes).unwrap();

    //     assert_eq!(recovered_message, original_message);
    // }

    #[test]
    /// Testeo de la funcion encriptar
    fn test_encrypt() {
        let content = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let encrypted_content = encrypt_3des(&content);

        assert_ne!(content.to_vec(), encrypted_content);
    }

    #[test]
    /// Testeo de la funcion desencriptar
    fn test_decrypt() {
        let content = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let encrypted_content = encrypt_3des(&content);
        let decrypted_content = decrypt_3des(&encrypted_content);

        assert_eq!(content.to_vec(), decrypted_content);
    }
}
