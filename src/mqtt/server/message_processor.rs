use std::sync::mpsc::{Receiver, Sender};

use rayon::ThreadPool;

use crate::mqtt::{
    messages::{
        packet_type::PacketType, puback_message::PubAckMessage, publish_message::PublishMessage,
        subscribe_message::SubscribeMessage,
    },
    mqtt_utils::utils::{display_debug_puback_msg, display_debug_publish_msg},
};

use std::io::Error;

use super::{mqtt_server_2::MQTTServer, packet::Packet};

#[derive(Debug)]
pub struct MessageProcessor {
    //rx_1: Receiver<Packet>,
    mqtt_server: MQTTServer,
    tx_2: Sender<Vec<u8>>,
}

impl MessageProcessor {
    pub fn new(mqtt_server: MQTTServer, tx_2: Sender<Vec<u8>>) -> Result<MessageProcessor, Error> {
        Ok(MessageProcessor { mqtt_server, tx_2 })
    }

    pub fn handle_packets(&mut self, rx_1: Receiver<Packet>) -> Result<(), Error> {
        self.set_tx_to_server();
        // Intenta crear un ThreadPool con 6 threads
        let pool = create_thread_pool(6);
        for packet in rx_1 {
            let self_clone = self.clone_ref();
            // Decide si usar el pool o no basado en si pool es Some o None.
            //Si es None continua el flujo del programa en el hilo actual
            match &pool {
                Some(pool) => {
                    pool.spawn(move || {
                        self_clone.process_packet(packet);
                    });
                }
                None => {
                    // Ejecuta la lógica de procesamiento directamente si no hay ThreadPool
                    self.process_packet(packet); // Ejecuta en el hilo actual
                }
            }
        }

        Ok(())
    }

    fn process_packet(&self, packet: Packet) {
        let msg_bytes = packet.get_msg_bytes();
        let username = packet.get_username();
        match packet.get_message_type() {
            PacketType::Publish => self.handle_publish(msg_bytes, &username),
            PacketType::Subscribe => self.handle_subscribe(msg_bytes, &username),
            PacketType::Puback => self.handle_puback(msg_bytes),
            _ => println!("   ERROR: Tipo de mensaje desconocido\n "),
        };
    }

    fn handle_publish(&self, msg_bytes: Vec<u8>, username: &str) {
        let publish_msg_res = PublishMessage::from_bytes(msg_bytes);
        match publish_msg_res {
            Ok(publish_msg) => {
                display_debug_publish_msg(&publish_msg);
                let puback_res = self.mqtt_server.send_puback(&publish_msg);
                if let Err(e) = puback_res {
                    println!("   ERROR: {:?}", e);
                    return;
                }
                println!(
                    "DEBUG: justo antes del handle_publish_message, para user: {:?}",
                    username
                );
                self.mqtt_server
                    .handle_publish_message(&publish_msg)
                    .unwrap();
                println!(
                    "DEBUG: justo dsp del handle_publish_message, para user: {:?}",
                    username
                );
            }
            Err(e) => println!("   ERROR: {:?}", e),
        }
    }

    fn handle_subscribe(&self, msg_bytes: Vec<u8>, username: &str) {
        let subscribe_msg_res = SubscribeMessage::from_bytes(msg_bytes);
        match subscribe_msg_res {
            Ok(msg) => {
                let return_codes_res = self.mqtt_server.add_topics_to_subscriber(username, &msg);
                match return_codes_res {
                    Ok(return_codes) => {
                        let operation_result =
                            self.mqtt_server.handle_subscribe_message(&msg, username);
                        if let Err(e) = operation_result {
                            println!("   ERROR: {:?}", e);
                            return;
                        }
                        let _ = self.mqtt_server.send_suback(return_codes);
                        println!("DEBUG: terminado el subscribe, para user: {:?}", username);
                    }
                    Err(e) => println!("   ERROR: {:?}", e),
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

    fn clone_ref(&self) -> Self {
        MessageProcessor {
            mqtt_server: self.mqtt_server.clone_ref(),
            tx_2: self.tx_2.clone(),
        }
    }

    fn set_tx_to_server(&mut self) {
        self.mqtt_server.set_tx(self.tx_2.clone());
    }
}

fn create_thread_pool(num_threads: usize) -> Option<ThreadPool> {
    match rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
    {
        Ok(pool) => Some(pool),
        Err(e) => {
            println!("No se pudo crear el threadpool: {:?}", e);
            None
        }
    }
}
