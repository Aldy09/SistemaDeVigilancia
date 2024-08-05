use std::io::Error;
use std::net::TcpStream;
use std::path::Path;

use crate::mqtt::messages::connack_message::ConnackMessage;
use crate::mqtt::messages::connack_session_present::SessionPresent;
use crate::mqtt::messages::connect_message::ConnectMessage;
use crate::mqtt::messages::connect_return_code::ConnectReturnCode;
use crate::mqtt::mqtt_utils::utils::write_message_to_stream;

use super::file_helper::read_lines;
use super::mqtt_server::MQTTServer;

#[derive(Debug)]
pub struct AuthenticateClient {}

impl AuthenticateClient {
    pub fn new() -> Self {
        AuthenticateClient {}
    }

    /// Procesa el mensaje de conexión recibido, autentica al cliente y envía un mensaje de conexión de vuelta.
    pub fn is_it_a_valid_connection(
        &self,
        connect_msg: &ConnectMessage,
        stream: &mut TcpStream,
        mqtt_server: &MQTTServer,
    ) -> Result<bool, Error> {
        let (is_authentic, connack_response) =
            self.was_the_session_created_succesfully(connect_msg)?;

        self.send_connection_response(&connack_response, stream)?;

        if is_authentic {
            self.handle_successful_authentication(connect_msg, stream, mqtt_server)
        } else {
            self.handle_failed_authentication();
            Ok(false)
        }
    }

    fn send_connection_response(
        &self,
        connack_response: &ConnackMessage,
        stream: &mut TcpStream,
    ) -> Result<(), Error> {
        write_message_to_stream(&connack_response.to_bytes(), stream)
    }

    fn handle_successful_authentication(
        &self,
        connect_msg: &ConnectMessage,
        stream: &mut TcpStream,
        mqtt_server: &MQTTServer,
    ) -> Result<bool, Error> {
        if let Some(username) = connect_msg.get_client_id() {
            let is_reconnection =
                mqtt_server.manage_possible_reconnecting_or_duplicate_user(username, stream)?;
            if !is_reconnection {
                println!(
                    "DEBUG: Agregando nuevo usuario al servidor con username {:?}",
                    username
                );
                mqtt_server.add_new_user(stream, username, connect_msg)?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn handle_failed_authentication(&self) {
        println!("   ERROR: No se pudo autenticar al cliente.");
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
        let credentials = self.read_credentials_from_file("credentials.txt");
        self.verify_authentication(user, passwd, &credentials)
    }

    /// Lee las credenciales del archivo especificado y devuelve un vector de pares (usuario, contraseña)
    fn read_credentials_from_file(&self, file_path: &str) -> Vec<(String, String)> {
        let path = Path::new(file_path);
        let mut credentials = Vec::new();

        if let Ok(lines) = read_lines(path) {
            for line in lines.map_while(Result::ok) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 2 {
                    credentials.push((parts[0].to_string(), parts[1].to_string()));
                }
            }
        }

        credentials
    }

    /// Verifica si el usuario y la contraseña proporcionados coinciden con alguna de las credenciales almacenadas
    fn verify_authentication(
        &self,
        user: Option<&String>,
        passwd: Option<&String>,
        credentials: &[(String, String)],
    ) -> bool {
        if let (Some(u), Some(p)) = (user, passwd) {
            credentials
                .iter()
                .any(|(username, password)| u == username && p == password)
        } else {
            false
        }
    }
}


impl Default for AuthenticateClient {
    fn default() -> Self {
        AuthenticateClient::new()
    }
}