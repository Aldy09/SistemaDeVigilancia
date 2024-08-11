use std::sync::mpsc::Receiver;

//use rayon::ThreadPool;

use crate::mqtt::{
    messages::{
        packet_type::PacketType, puback_message::PubAckMessage, publish_message::PublishMessage,
        subscribe_message::SubscribeMessage, subscribe_return_code::SubscribeReturnCode,
    },
    mqtt_utils::utils::{display_debug_puback_msg, display_debug_publish_msg},
};

use std::io::Error;

use super::{mqtt_server::MQTTServer, packet::Packet};

#[derive(Debug)]
pub struct MessageProcessor {
    mqtt_server: MQTTServer,
}

impl MessageProcessor {
    pub fn new(mqtt_server: MQTTServer) -> Self {
        MessageProcessor { mqtt_server }
    }

    pub fn handle_packets(&mut self, rx_1: Receiver<Packet>) -> Result<(), Error> {
        for packet in rx_1 {
            self.process_packet(packet); // Ejecuta en el hilo actual
        }

        Ok(())
    }

    fn process_packet(&self, packet: Packet) {
        let msg_bytes = packet.get_msg_bytes();
        let client_id = packet.get_username();
        match packet.get_message_type() {
            PacketType::Publish => self.handle_publish(msg_bytes, client_id),
            PacketType::Subscribe => self.handle_subscribe(msg_bytes, client_id),
            PacketType::Puback => self.handle_puback(msg_bytes),
            _ => println!("   ERROR: Tipo de mensaje desconocido\n "),
        };
    }

    fn handle_publish(&self, msg_bytes: Vec<u8>, client_id: &str) {
        let publish_msg_res = PublishMessage::from_bytes(msg_bytes);
        match publish_msg_res {
            Ok(publish_msg) => {
                display_debug_publish_msg(&publish_msg);
                let puback_res = self.send_puback_to(client_id, &publish_msg);
                if let Err(e) = puback_res {
                    println!("   ERROR: {:?}", e);
                    return;
                }
                self.mqtt_server
                    .handle_publish_message(&publish_msg)
                    .unwrap();
            }
            Err(e) => println!("   ERROR: {:?}", e),
        }
    }

    fn handle_subscribe(&self, msg_bytes: Vec<u8>, client_id: &str) {
        let subscribe_msg_res = SubscribeMessage::from_bytes(msg_bytes);
        match subscribe_msg_res {
            Ok(msg) => {
                let return_codes_res = self.mqtt_server.add_topics_to_subscriber(client_id, &msg);
                let operation_result = self
                    .mqtt_server
                    .send_preexisting_msgs_to_new_subscriber(client_id, &msg);
                if let Err(e) = operation_result {
                    println!("   ERROR: {:?}", e);
                }
                let suback_res = self.send_suback_to(client_id, return_codes_res);
                if let Err(e) = suback_res {
                    println!("   ERROR: {:?}", e);
                }
            }
            Err(e) => println!("   ERROR: {:?}", e),
        }
    }

    fn handle_puback(&self, msg_bytes: Vec<u8>) {
        let puback_msg_res = PubAckMessage::msg_from_bytes(msg_bytes);
        match puback_msg_res {
            Ok(puback_msg) => display_debug_puback_msg(&puback_msg),
            Err(e) => println!("   ERROR: {:?}", e),
        }
    }

    pub fn send_puback_to(
        &self,
        client_id: &str,
        publish_msg: &PublishMessage,
    ) -> Result<(), Error> {
        self.mqtt_server.send_puback_to(client_id, publish_msg)?;

        Ok(())
    }

    fn send_suback_to(
        &self,
        client_id: &str,
        return_codes_res: Result<Vec<SubscribeReturnCode>, Error>,
    ) -> Result<(), Error> {
        self.mqtt_server
            .send_suback_to(client_id, &return_codes_res)?;
        Ok(())
    }
}

// fn clone_ref(&self) -> Self {
//     MessageProcessor {
//         mqtt_server: self.mqtt_server.clone_ref(),
//     }
// }

// fn create_thread_pool(num_threads: usize) -> Option<ThreadPool> {
//     match rayon::ThreadPoolBuilder::new()
//         .num_threads(num_threads)
//         .build()
//     {
//         Ok(pool) => Some(pool),
//         Err(e) => {
//             println!("No se pudo crear el threadpool: {:?}", e);
//             None
//         }
//     }
// }
