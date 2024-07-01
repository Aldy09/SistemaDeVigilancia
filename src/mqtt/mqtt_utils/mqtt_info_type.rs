use std::sync::mpsc::Receiver;

use crate::mqtt::{client::mqtt_client::MQTTClient, messages::publish_message::PublishMessage};

pub type MQTTInfo = (MQTTClient, Receiver<PublishMessage>);