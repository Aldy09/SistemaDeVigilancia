use std::sync::mpsc::{Receiver, Sender};

use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::mqtt::{
    self,
    messages::{
        packet_type::PacketType, puback_message::PubAckMessage, publish_message::PublishMessage,
        subscribe_message::SubscribeMessage,
    },
    mqtt_utils::utils::{display_debug_puback_msg, display_debug_publish_msg},
};

use std::io::{Error, ErrorKind, Write};

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

    pub fn handle_packets(&self, rx_1: Receiver<Packet>) -> Result<(), Error> {
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
            PacketType::Publish => {
                // Publish
                let publish_msg_res = PublishMessage::from_bytes(msg_bytes);
                match publish_msg_res {
                    Ok(publish_msg) => {
                        display_debug_publish_msg(&publish_msg);
                        let puback_res = self.mqtt_server.send_puback(&publish_msg);
                        if puback_res.is_err() {
                            println!("   ERROR: {:?}", puback_res.err().unwrap());
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
                    Err(e) => {
                        println!("   ERROR: {:?}", e);
                        return;
                    }
                }
            }
            PacketType::Subscribe => {
                // Subscribe
                let subsribe_msg_res = SubscribeMessage::from_bytes(msg_bytes);
                match subsribe_msg_res {
                    Ok(msg) => {
                        let return_codes_res =
                            self.mqtt_server.add_topics_to_subscriber(username, &msg);
                        match return_codes_res {
                            Ok(return_codes) => {
                                self.handle_subscribe_message(&msg, username);

                                let _ = self.mqtt_server.send_suback(return_codes);
                                println!(
                                    "DEBUG: terminado el subscribe, para user: {:?}",
                                    username
                                );
                            }
                            Err(e) => {
                                println!("   ERROR: {:?}", e);
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        println!("   ERROR: {:?}", e);
                        return;
                    }
                };
            }
            PacketType::Puback => {
                // PubAck
                let puback_msg_res = PubAckMessage::msg_from_bytes(msg_bytes);
                match puback_msg_res {
                    Ok(puback_msg) => {
                        display_debug_puback_msg(&puback_msg);
                    }
                    Err(e) => {
                        println!("   ERROR: {:?}", e);
                        return;
                    }
                }
            }
            _ => println!("   ERROR: Tipo de mensaje desconocido\n "),
        };
    }

    fn handle_subscribe_message(&self, msg: &SubscribeMessage, username: &str) {
        // Obtiene el topic al que se está suscribiendo el user
        for (topic, _) in msg.get_topic_filters() {
            if self.mqtt_server.there_are_old_messages_to_send_for(topic) {
                // Al user que se conecta, se le envía lo que no tenía del topic en cuestión
                if let Ok(mut connected_users_locked) = self.connected_users.lock() {
                    if let Some(user) = connected_users_locked.get_mut(username) {
                        // Necesitamos también los mensajes
                        if let Ok(mut messages_by_topic_locked) =
                            self.mqtt_server.messages_by_topic.lock()
                        {
                            if let Some(topic_messages) = messages_by_topic_locked.get_mut(topic) {
                                self.mqtt_server.send_unreceived_messages(
                                    user,
                                    topic,
                                    topic_messages,
                                )?;
                            }
                        } else {
                            return Err(Error::new(
                                    ErrorKind::Other,
                                    "Error: no se pudo tomar lock a messages_by_topic para enviar Publish durante un Subscribe."));
                        }
                    }
                }
            }
        }
    }

    fn clone_ref(&self) -> Self {
        MessageProcessor {
            mqtt_server: self.mqtt_server.clone(),
            tx_2: self.tx_2.clone(),
        }
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
