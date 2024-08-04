use std::io::Error;
use std::net::TcpStream;
use std::path::Path;

use crate::mqtt::messages::connack_message::ConnackMessage;
use crate::mqtt::messages::connack_session_present::SessionPresent;
use crate::mqtt::messages::connect_message::ConnectMessage;
use crate::mqtt::messages::connect_return_code::ConnectReturnCode;
use crate::mqtt::mqtt_utils::utils::write_message_to_stream;

use super::file_helper::read_lines;
use super::mqtt_server_2::MQTTServer;

#[derive(Debug)]
pub struct AuthenticateClient {}

impl AuthenticateClient {
    pub fn new() -> Self {
        AuthenticateClient {}
    }

    /// Procesa el mensaje de conexión recibido, autentica
    /// al cliente y envía un mensaje de conexión de vuelta.
    pub fn is_it_a_valid_connection(
        &self,
        connect_msg: &ConnectMessage,
        stream: &mut TcpStream,
        mqtt_server: &MQTTServer,
    ) -> Result<bool, Error> {
        // Procesa el mensaje connect
        let (is_authentic, connack_response) =
            self.was_the_session_created_succesfully(&connect_msg)?;

        //mqtt_server.send_connack(connack_response);
        write_message_to_stream(&connack_response.to_bytes(), stream)?;

        // Si el cliente se autenticó correctamente y se conecta por primera vez,
        // se agrega a la lista de usuarios conectados y se maneja la conexión
        if is_authentic {
            if let Some(username) = connect_msg.get_client_id() {
                // Busca en el hashmap, contempla casos si ya existía ese cliente:
                // lo desconecta si es duplicado, le envía lo que no tiene si se reconecta.
                let mut is_reconnection = false;
                if let Some(client_id) = connect_msg.get_client_id() {
                    is_reconnection = mqtt_server
                        .manage_possible_reconnecting_or_duplicate_user(client_id, &stream)?;
                }

                // Si es reconexión, no quiero add_new_user xq eso me crearía un User nuevo y yo ya tengo uno.
                if !is_reconnection {
                    mqtt_server.add_new_user(&stream, username, &connect_msg)?;
                }

                return Ok(true);
            }
        } else {
            println!("   ERROR: No se pudo autenticar al cliente."); //(ya se le envió el ack informando).
        }

        Ok(false)
    }

    /// Verifica si la sesión fue creada exitosamente: usuario valido o invitado
    /// y devuelve un mensaje CONNACK acorde.
    fn was_the_session_created_succesfully(
        &self,
        connect_msg: &ConnectMessage,
    ) -> Result<(bool, ConnackMessage), Error> {
        if self.is_guest_mode_active(connect_msg.get_user(), connect_msg.get_passwd())
            || self.authenticate(connect_msg.get_user(), connect_msg.get_passwd())
        {
            // Aux: volver cuando haya tema desconexiones, acá le estoy pasando siempre un SessionPresent::NotPresentInLastSession. [].
            let connack_response = ConnackMessage::new(
                SessionPresent::NotPresentInLastSession,
                ConnectReturnCode::ConnectionAccepted,
            );
            Ok((true, connack_response))
        } else {
            let connack_response = ConnackMessage::new(
                SessionPresent::NotPresentInLastSession,
                ConnectReturnCode::NotAuthorized,
            );
            Ok((false, connack_response))
        }
    }

    fn is_guest_mode_active(&self, user: Option<&String>, passwd: Option<&String>) -> bool {
        user.is_none() && passwd.is_none()
    }

    /// Autentica al usuario con las credenciales almacenadas en el archivo credentials.txt
    fn authenticate(&self, user: Option<&String>, passwd: Option<&String>) -> bool {
        let mut is_authentic: bool = false;
        let credentials_path = Path::new("credentials.txt");
        if let Ok(lines) = read_lines(credentials_path) {
            for line in lines.map_while(Result::ok) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() != 2 {
                    continue;
                }
                let username = parts[0]; // username
                let password = parts[1]; // password
                if let Some(msg_user) = user {
                    if let Some(msg_passwd) = passwd {
                        is_authentic = msg_user == username && msg_passwd == password;
                        if is_authentic {
                            break;
                        }
                    }
                }
            }
        }
        is_authentic
    }
}
