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

use super::{mqtt_server_2::MQTTServer, packet::Packet};

#[derive(Debug)]
pub struct MessageProcessor {
    mqtt_server: MQTTServer,
}

impl MessageProcessor {
    pub fn new(mqtt_server: MQTTServer) -> Result<MessageProcessor, Error> {
        Ok(MessageProcessor { mqtt_server })
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
        let message_type = packet.get_message_type();
        println!("DEBUG: RECIBIENDO PAQUETE DE TIPO {:?} PARA EL USUARIO: {:?}", message_type, client_id);
        match packet.get_message_type() {
            PacketType::Publish => self.handle_publish(msg_bytes, &client_id),
            PacketType::Subscribe => self.handle_subscribe(msg_bytes, &client_id),
            PacketType::Puback => self.handle_puback(msg_bytes),
            _ => println!("   ERROR: Tipo de mensaje desconocido\n "),
        };
    }

    fn handle_publish(&self, msg_bytes: Vec<u8>, client_id: &str) {
        let publish_msg_res = PublishMessage::from_bytes(msg_bytes);
        match publish_msg_res {
            Ok(publish_msg) => {
                display_debug_publish_msg(&publish_msg);
                let puback_res = self.send_puback_to(&publish_msg, client_id);
                if let Err(e) = puback_res {
                    println!("   ERROR: {:?}", e);
                    return;
                }
                println!(
                    "DEBUG: justo antes del handle_publish_message, para user: {:?}",
                    client_id
                );
                self.mqtt_server
                    .handle_publish_message(&publish_msg)
                    .unwrap();
                println!(
                    "DEBUG: justo dsp del handle_publish_message, para user: {:?}",
                    client_id
                );
            }
            Err(e) => println!("   ERROR: {:?}", e),
        }
    }

    fn handle_subscribe(&self, msg_bytes: Vec<u8>, client_id: &str) {
        let subscribe_msg_res = SubscribeMessage::from_bytes(msg_bytes);
        match subscribe_msg_res {
            Ok(msg) => {
                println!("DEBUG: justo antes del add_topics_to_subscriber, para user: {:?}", client_id);
                let return_codes_res = self.mqtt_server.add_topics_to_subscriber(client_id, &msg);
                let operation_result = self.mqtt_server.handle_subscribe_message(&msg, client_id);
                if let Err(e) = operation_result {
                    println!("   ERROR: {:?}", e);
                }
                let suback_res = self.send_suback_to(return_codes_res, client_id);
                if let Err(e) = suback_res {
                    println!("   ERROR: {:?}", e);
                }
                println!("DEBUG: Terminado el suscribe para el usuario: {:?}", client_id);
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

    
    pub fn send_puback_to(&self, publish_msg: &PublishMessage, client_id: &str) -> Result< (), Error> {
        if let Ok(mut connected_users_locked) = self.mqtt_server.get_connected_users().lock() {
            if let Some(user) = connected_users_locked.get_mut(client_id) {
                self.mqtt_server.send_puback(publish_msg, &mut user.get_stream()?)?;
            }
        }
        Ok(())
    }
    
    fn send_suback_to(&self, return_codes_res: Result<Vec<SubscribeReturnCode>, Error>, client_id: &str,) -> Result< (), Error > {

        if let Ok(mut connected_users_locked) = self.mqtt_server.get_connected_users().lock() {
            if let Some(user) = connected_users_locked.get_mut(client_id) {
                self.mqtt_server.send_suback(&return_codes_res, &mut user.get_stream()?)?;
            }
        }
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
