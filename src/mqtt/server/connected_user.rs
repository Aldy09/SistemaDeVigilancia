use std::{
    collections::{HashMap, VecDeque},
    io::Error,
    net::TcpStream,
    sync::{Arc, Mutex},
};

//use crate::mqtt::mqtt_utils::stream_type::StreamType;
type StreamType = TcpStream;
use crate::mqtt::messages::publish_message::PublishMessage;

use super::user_state::UserState;

type ShareableMessageQueue = Arc<Mutex<HashMap<String, VecDeque<PublishMessage>>>>;

/// Representa a un usuario (cliente) conectado al MQTTServer, del lado del servidor.
#[derive(Debug)]
#[allow(dead_code)]

pub struct User {
    stream: StreamType,
    username: String,
    state: UserState,
    topics: Vec<String>, //topics a los que esta suscripto
    messages: Arc<Mutex<HashMap<String, VecDeque<PublishMessage>>>>, // por cada topic tiene una cola de mensajes tipo publish
}

impl User {
    pub fn new(stream: StreamType, username: String) -> Self {
        User {
            stream,
            username,
            state: UserState::Active,
            topics: Vec::new(),
            messages: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    // Getters
    pub fn get_stream(&self) -> Result<StreamType, Error> {
        self.stream.try_clone()
    }

    pub fn get_username(&self) -> String {
        self.username.to_string()
    }
    
    pub fn is_not_disconnected(&self) -> bool {
        self.state != UserState::TemporallyDisconnected
    }
    
    pub fn get_state(&self) -> &UserState {
        &self.state
    }

    pub fn get_topics(&self) -> &Vec<String> {
        &self.topics
    }

    pub fn get_hashmap_messages(&self) -> &ShareableMessageQueue {
        &self.messages
    }

    // Setters
    pub fn set_stream(&mut self, stream: StreamType) {
        self.stream = stream;
    }
    
    pub fn set_state(&mut self, state: UserState) {
        self.state = state;
    }

    pub fn set_username(&mut self, username: String) {
        self.username = username;
    }

    pub fn add_topic(&mut self, topic: String) {
        self.topics.push(topic);
    }

    pub fn set_messages(&mut self, messages: ShareableMessageQueue) {
        self.messages = messages;
    }

    /// Agrega el mensaje a la cola del usuario
    pub fn add_message_to_queue(&mut self, message: PublishMessage) {
        let topic = message.get_topic();
        if let Ok(mut messages_locked) = self.messages.lock() {
            messages_locked
                .entry(topic)
                .or_default() //si no existe el topic, lo crea con el valor: ""VecDeque::new()"" a checkear
                //.or_insert_with(VecDeque::new)
                .push_back(message);
        }
    }
    
}
