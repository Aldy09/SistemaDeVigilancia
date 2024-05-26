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
    pub username: String,
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
}
