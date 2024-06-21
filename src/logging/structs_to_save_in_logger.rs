use crate::{apps::app_type::AppType, mqtt::messages::message_type::MessageType};

#[derive(Debug)]
pub enum OperationType {
    Sent,
    Received,
}

#[derive(Debug)]
pub enum StructsToSaveInLogger {
    AppType(String, AppType, OperationType),
    MessageType(String, MessageType, OperationType),
}
// pub enum StructsToSaveInLogger {
//     AppType(AppType),
//     MessageType(MessageType),
// }
