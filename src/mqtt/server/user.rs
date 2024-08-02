use std::{
    collections::{HashMap, VecDeque},
    io::Error,
    net::TcpStream,
    sync::{Arc, Mutex},
};

//use crate::mqtt::mqtt_utils::stream_type::StreamType;
type StreamType = TcpStream;
use crate::mqtt::{messages::{publish_flags::PublishFlags, publish_message::PublishMessage},
           mqtt_utils::will_message_utils::will_message::WillMessageData};

use super::user_state::UserState;

type ShareableMessageQueue = Arc<Mutex<HashMap<String, VecDeque<PublishMessage>>>>;

/// Representa a un usuario (cliente) conectado al MQTTServer, del lado del servidor.
#[derive(Debug)]
#[allow(dead_code)]

pub struct User {
    stream: StreamType,
    username: String,
    state: UserState,
    will_message: Option<WillMessageData>,
    topics: Vec<String>, //topics a los que esta suscripto
    messages: Arc<Mutex<HashMap<String, VecDeque<PublishMessage>>>>, // por cada topic tiene una cola de mensajes tipo publish
    last_id_by_topic: HashMap<String, u8>, // por cada topic tiene el ultimo id de mensaje enviado
}

impl User {
    pub fn new(stream: StreamType, username: String, will_msg_and_topic: Option<WillMessageData>) -> Self {
        User {
            stream,
            username,
            state: UserState::Active,
            will_message: will_msg_and_topic,
            topics: Vec::new(),
            messages: Arc::new(Mutex::new(HashMap::new())),
            last_id_by_topic: HashMap::new(),
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

    /*pub fn get_will_message(&self) -> PublishMessage {
        self.will_msg;
    }*/
    /*pub fn get_will_message_and_topic(&self) -> Option<WillMessageAndTopic> {
        self.will_message
    }*/
    pub fn get_publish_message_with(&self, dup_flag: u8, packet_id: u16) -> Result<Option<PublishMessage>, Error> {
        if let Some(info) = &self.will_message {
            let flags = PublishFlags::new(dup_flag, info.get_qos(), info.get_will_retain())?;
            let publish_msg = PublishMessage::new(
                3,
                flags,
                &info.get_will_topic(),
                Some(packet_id),
                info.get_will_msg_content().as_bytes())?;
            
            return Ok(Some(publish_msg));

        }
        
        Ok(None)
    }


    pub fn is_last_outdated(&self, last_id_topic_server: u8, topic: &String) -> bool {
        if let Some(last_id) = self.last_id_by_topic.get(topic) {
            return *last_id < last_id_topic_server;
        }
        false // si no existe el topic, devuelve false, pero no deberia darse este caso
    }


    pub fn get_last_id_by_topic(&self, topic: &String) -> u8 {
        if let Some(last_id) = self.last_id_by_topic.get(topic) {
            return *last_id;
        }
        0
    }

    pub fn update_last_id_by_topic(&mut self, topic: &String, last_id: u8) {
        self.last_id_by_topic.insert(topic.clone(), last_id);
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
        self.topics.push(topic.clone());
        self.last_id_by_topic.insert(topic, 0);
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
