use super::packet_type::PacketType;
use std::any::Any;

//pub trait Message: Send {
pub trait Message: Send + Any {   
    fn get_packet_id(&self) -> Option<u16>;

    fn to_bytes(&self) -> Vec<u8>;

    fn get_type(&self) -> PacketType;

    fn as_any(&self) -> &dyn Any;
}