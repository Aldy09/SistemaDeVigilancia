use std::{
    collections::{HashMap, VecDeque},
    net::TcpStream,
    sync::{Arc, Mutex},
};

use crate::publish_message::PublishMessage;

#[derive(Debug)]
#[allow(dead_code)]

pub struct User {
    stream: Arc<Mutex<TcpStream>>,
    username: String,
    topics: Vec<String>, //topics a los que esta suscripto
    messages: HashMap<String, VecDeque<PublishMessage>>,
}

impl User {
    pub fn new(stream: Arc<Mutex<TcpStream>>, username: String) -> Self {
        User {
            stream,
            username,
            topics: Vec::new(),
            messages: HashMap::new(),
        }
    }
    // Getters
    pub fn get_stream(&self) -> Arc<Mutex<TcpStream>> {
        Arc::clone(&self.stream)
    }

    pub fn get_username(&self) -> &String {
        &self.username
    }

    pub fn get_topics(&self) -> &Vec<String> {
        &self.topics
    }

    pub fn get_messages(&self) -> &HashMap<String, VecDeque<PublishMessage>> {
        &self.messages
    }

    // Setters
    pub fn set_stream(&mut self, stream: Arc<Mutex<TcpStream>>) {
        self.stream = stream;
    }

    pub fn set_username(&mut self, username: String) {
        self.username = username;
    }

    pub fn add_to_topics(&mut self, topic:String) {
        self.topics.push(topic);
    }

    pub fn set_messages(&mut self, messages: HashMap<String, VecDeque<PublishMessage>>) {
        self.messages = messages;
    }
}
