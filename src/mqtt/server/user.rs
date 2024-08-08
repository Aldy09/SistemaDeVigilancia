use std::{collections::HashMap, fmt::write, io::{Error, Write}, net::TcpStream};

//use crate::mqtt::mqtt_utils::stream_type::StreamType;
type StreamType = TcpStream;
use crate::mqtt::{
    messages::{publish_flags::PublishFlags, publish_message::PublishMessage},
    mqtt_utils::will_message_utils::will_message::WillMessageData,
};

use super::user_state::UserState;

/// Representa a un usuario (cliente) conectado al MQTTServer, del lado del servidor.
#[derive(Debug)]
#[allow(dead_code)]

pub struct User {
    username: String, // se identifica por el username.
    stream: StreamType,
    state: UserState,
    will_message: Option<WillMessageData>,
    topics: Vec<String>,                    // topics a los que esta suscripto
    last_id_by_topic: HashMap<String, u32>, // por cada topic tiene el ultimo id de mensaje enviado.
}

impl User {
    /// Crea un User.
    pub fn new(
        stream: StreamType,
        username: String,
        will_msg_and_topic: Option<WillMessageData>,
    ) -> Self {
        User {
            username,
            stream,
            state: UserState::Active,
            will_message: will_msg_and_topic,
            topics: Vec::new(),
            last_id_by_topic: HashMap::new(),
        }
    }

    /// Devuelve el stream del user.
    pub fn get_stream(&self) -> Result<StreamType, Error> {
        self.stream.try_clone()
    }

    /// Devuelve el username del user.
    pub fn get_username(&self) -> String {
        self.username.to_string()
    }

    /// Devuelve si el user no está desconectado.
    pub fn is_not_disconnected(&self) -> bool {
        self.state != UserState::TemporallyDisconnected
    }

    /// Devuelve el estado del user.
    pub fn get_state(&self) -> &UserState {
        &self.state
    }

    /// Crea el PublishMessage necesario para publicar el will message que User tiene almacenado desde el principio de la conexión.
    pub fn get_publish_message_with(
        &self,
        dup_flag: u8,
        packet_id: u16,
    ) -> Result<Option<PublishMessage>, Error> {
        if let Some(info) = &self.will_message {
            let flags = PublishFlags::new(dup_flag, info.get_qos(), info.get_will_retain())?;
            let publish_msg = PublishMessage::new(
                3,
                flags,
                &info.get_will_topic(),
                Some(packet_id),
                info.get_will_msg_content().as_bytes(),
            )?;

            return Ok(Some(publish_msg));
        }

        Ok(None)
    }

    /// Devuelve el last_id para el topic `topic`.
    pub fn get_last_id_by_topic(&self, topic: &String) -> u32 {
        if let Some(last_id) = self.last_id_by_topic.get(topic) {
            return *last_id;
        }
        println!("PRINT DEBUG PROBANDO"); // []
        0 // <---. // Aux. Por qué podría no encontrarse el topic? xq no se insertó todavía.., pero si se está reconectando debería existir de la conexión anterior;
    }

    /// Actualiza (sobreescribe) el `last_id` del topic `topic`, con el recibido por parámetro.
    pub fn update_last_id_by_topic(&mut self, topic: &String, last_id: u32) {
        self.last_id_by_topic.insert(topic.to_owned(), last_id);
    }

    /// Devuelve los topics a los que el user está suscripto.
    pub fn get_topics(&self) -> &Vec<String> {
        &self.topics
    }

    /// Se guarda el nuevo stream, después de una reconexión.
    pub fn update_stream_with(&mut self, new_stream: StreamType) {
        self.stream = new_stream
    }

    /// Setea el estado del user.
    pub fn set_state(&mut self, state: UserState) {
        self.state = state;
    }

    /// Agrega el topic a los topics a los que user está suscripto.
    pub fn add_topic(&mut self, topic: String) {
        self.topics.push(topic.clone());
        // Inicializa su last_id para ese topic en 0 si el mismo no existía.
        self.last_id_by_topic.entry(topic).or_insert(0);
    }

    /// Escribe el mensaje en bytes `msg_bytes` por el stream hacia el cliente.
    /// Puede devolver error si falla la escritura o el flush.
    pub fn write_message(&mut self, msg_bytes: &[u8]) -> Result<(), Error> {
        if self.is_not_disconnected() {
            let _ = self.stream.write(msg_bytes)?;
            self.stream.flush()?;
            return Ok(());
        } 
        Err(Error::new(std::io::ErrorKind::InvalidInput, "Error: User no conectado"))
    }
}
