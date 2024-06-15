use super::{publish_message::PublishMessage, subscribe_message::SubscribeMessage};

#[derive(Debug)]
pub enum MessageType {
    Subscribe(SubscribeMessage),
    Publish(PublishMessage),
}