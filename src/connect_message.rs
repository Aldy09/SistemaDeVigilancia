#[derive(Debug)]
pub struct ConnectMessage<'a> {
    client_id: &'a str,
    clean_session: bool,
    keep_alive: u16,
    username: Option<&'a str>,
    password: Option<&'a str>,
}

impl<'a> ConnectMessage<'a> {
    pub fn new(client_id: &'a str, clean_session: bool, keep_alive: u16, username: Option<&'a str>, password: Option<&'a str>) -> Self {
        Self {
            client_id,
            clean_session,
            keep_alive,
            username,
            password,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut connect_flags: u8 = 0;
        if self.clean_session {
            connect_flags |= 0x02; // Bit 1: Clean Session Flag
        }
        if self.username.is_some() {
            connect_flags |= 0x80; // Bit 7: Username Flag
        }
        if self.password.is_some() {
            connect_flags |= 0x40; // Bit 6: Password Flag
        }

        let mut connect_msg: Vec<u8> = Vec::new();
        connect_msg.push(0x10); // Tipo de mensaje CONNECT
        connect_msg.push(0x00); // Longitud del mensaje CONNECT (se actualiza luego)
        connect_msg.extend_from_slice("MQTT".as_bytes()); // Protocolo "MQTT"
        connect_msg.push(0x04); // Versión 4 del protocolo MQTT
        connect_msg.push(connect_flags); // Flags de CONNECT
        connect_msg.push((self.keep_alive >> 8) as u8); // Keep Alive MSB
        connect_msg.push((self.keep_alive & 0xFF) as u8); // Keep Alive LSB
        connect_msg.extend_from_slice(self.client_id.as_bytes()); // Client ID

        if let Some(username) = self.username {
            connect_msg.extend_from_slice(username.as_bytes());
        }
        if let Some(password) = self.password {
            connect_msg.extend_from_slice(password.as_bytes());
        }

        // Actualiza la longitud del mensaje CONNECT
        let msg_len = connect_msg.len() - 2;
        connect_msg[1] = msg_len as u8;

        connect_msg
    }
}
