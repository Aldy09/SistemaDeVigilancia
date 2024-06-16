use crate::{apps::app_type::AppType, messages::message_type::MessageType};

#[derive(Debug)]
pub enum StructsToSaveInLogger {
    AppType(AppType),
    MessageType(MessageType),
}
