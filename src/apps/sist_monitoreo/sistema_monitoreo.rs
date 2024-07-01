use std::{
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

use crossbeam_channel::Receiver as CrossbeamReceiver;
use crossbeam_channel::Sender as CrossbeamSender;

use std::sync::mpsc::{Receiver as MpscReceiver, Sender as MpscSender};

use crate::{apps::common_clients::is_disconnected_error, logging::{
    logger::Logger,
    structs_to_save_in_logger::{OperationType, StructsToSaveInLogger},
}};

use crate::mqtt::{
    client::mqtt_client::MQTTClient,
    messages::{message_type::MessageType, publish_message::PublishMessage},
};

use super::ui_sistema_monitoreo::UISistemaMonitoreo;
use crate::apps::{
    app_type::AppType,
    common_clients::exit_when_asked,
    incident::Incident,
};

#[derive(Debug)]
pub struct SistemaMonitoreo {
    pub incidents: Arc<Mutex<Vec<Incident>>>,
    pub logger_tx: MpscSender<StructsToSaveInLogger>,
    pub egui_tx: CrossbeamSender<PublishMessage>,
}

impl SistemaMonitoreo {
    pub fn new(logger_tx: MpscSender<StructsToSaveInLogger>, egui_tx: CrossbeamSender<PublishMessage>) -> Self {
        
        let sistema_monitoreo: SistemaMonitoreo = Self {
            incidents: Arc::new(Mutex::new(Vec::new())),
            logger_tx,
            egui_tx
        };

        sistema_monitoreo
    }

    pub fn spawn_threads(
        &self,
        logger_rx: MpscReceiver<StructsToSaveInLogger>,
        publish_message_rx: MpscReceiver<PublishMessage>, egui_rx: CrossbeamReceiver<PublishMessage>, mqtt_client: MQTTClient
    ) -> Vec<JoinHandle<()>> {
        
        let (incident_tx, incident_rx) = mpsc::channel::<Incident>();
        let (exit_tx, exit_rx) = mpsc::channel::<bool>();

        let logger = Logger::new(logger_rx);
        let mut children: Vec<JoinHandle<()>> = vec![];
        let mqtt_client_sh = Arc::new(Mutex::new(mqtt_client));
        let mqtt_client_incident_sh_clone = Arc::clone(&mqtt_client_sh.clone());

        children.push(self.spawn_subscribe_to_topics_thread(mqtt_client_sh.clone(), publish_message_rx));

        // Thread para hacer publish de un incidente que llega atraves de la UI
        children.push(
            self
                .spawn_publish_incidents_in_topic_thread(mqtt_client_incident_sh_clone.clone(), incident_rx),
        );


        children.push(spawn_write_incidents_to_logger_thread(logger));

        children.push(
            self.spawn_exit_thread(mqtt_client_incident_sh_clone.clone(), exit_rx),
        );

        self.spawn_ui_thread(incident_tx, egui_rx, exit_tx);

        children
    }

    pub fn spawn_ui_thread(
        &self,
        incident_tx: MpscSender<Incident>,
        publish_message_rx: CrossbeamReceiver<PublishMessage>,
        exit_tx: MpscSender<bool>,
    ) {
        let _ = eframe::run_native(
            "Sistema Monitoreo",
            Default::default(),
            Box::new(|cc| {
                Box::new(UISistemaMonitoreo::new(
                    cc.egui_ctx.clone(),
                    incident_tx,
                    publish_message_rx,
                    exit_tx,
                ))
            }),
        );
    }

    pub fn spawn_publish_incidents_in_topic_thread(
        &self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        rx: MpscReceiver<Incident>,
    ) -> JoinHandle<()> {
        let self_clone = self.clone_ref();
        thread::spawn(move || loop {
            while let Ok(msg) = rx.recv() {
                let msg_clone = msg.clone();
                self_clone
                    .logger_tx
                    .send(StructsToSaveInLogger::AppType(
                        "Sistema Monitoreo".to_string(),
                        AppType::Incident(msg),
                        OperationType::Sent,
                    ))
                    .unwrap();
                self_clone.publish_incident(msg_clone, &mqtt_client);
            }
        })
    }

    pub fn clone_ref(&self) -> Self {
        Self {
            incidents: self.incidents.clone(),
            egui_tx: self.egui_tx.clone(),
            logger_tx: self.logger_tx.clone(),
        }
    }

    pub fn spawn_subscribe_to_topics_thread(
        &self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        mqtt_rx: MpscReceiver<PublishMessage>
    ) -> JoinHandle<()> {
        let self_clone = self.clone_ref();
        thread::spawn(move || {
            self_clone.subscribe_to_topics(mqtt_client, mqtt_rx);
        })
    }

    pub fn subscribe_to_topics(&self, mqtt_client: Arc<Mutex<MQTTClient>>, mqtt_rx: MpscReceiver<PublishMessage>) {
        self.subscribe_to_topic(&mqtt_client, "Cam");
        self.subscribe_to_topic(&mqtt_client, "Dron");
        self.receive_messages_from_subscribed_topics(mqtt_rx);
    }

    pub fn subscribe_to_topic(&self, mqtt_client: &Arc<Mutex<MQTTClient>>, topic: &str) {
        if let Ok(mut mqtt_client) = mqtt_client.lock() {
            let res_sub = mqtt_client.mqtt_subscribe(vec![(String::from(topic))]);
            match res_sub {
                Ok(subscribe_message) => {
                    self.logger_tx
                        .send(StructsToSaveInLogger::MessageType(
                            "Sistema Monitoreo".to_string(),
                            MessageType::Subscribe(subscribe_message),
                            OperationType::Sent,
                        ))
                        .unwrap();
                }
                Err(e) => println!("Cliente: Error al hacer un subscribe a topic: {:?}", e),
            }
        }
    }

    // Recibe mensajes de los topics a los que se ha suscrito
    pub fn receive_messages_from_subscribed_topics(&self, mqtt_rx: MpscReceiver<PublishMessage>) {
        loop {
            match mqtt_rx.recv() {
                //Publish message: camera o dron
                Ok(publish_message) => {
                    self.logger_tx
                        .send(StructsToSaveInLogger::MessageType(
                            "Sistema Monitoreo".to_string(),
                            MessageType::Publish(publish_message.clone()),
                            OperationType::Received,
                        ))
                        .unwrap();
                    self.send_publish_message_to_ui(publish_message)
                }
                Err(_) => {
                    is_disconnected_error();
                    break;
                }
            }
        }
    }

    pub fn send_publish_message_to_ui(&self, msg: PublishMessage) {
        let res_send = self.egui_tx.send(msg);
        match res_send {
            Ok(_) => println!("Cliente: Enviado mensaje a la UI"),
            Err(e) => println!("Cliente: Error al enviar mensaje a la UI: {:?}", e),
        }
    }

    pub fn add_incident(&mut self, incident: Incident) {
        self.incidents.lock().unwrap().push(incident);
    }

    pub fn get_incidents(&mut self) -> Arc<Mutex<Vec<Incident>>> {
        self.incidents.clone()
    }

    pub fn generate_new_incident_id(&self) -> u8 {
        let mut new_inc_id: u8 = 0;
        if let Ok(incidents) = self.incidents.lock() {
            new_inc_id = (incidents.len() + 1) as u8;
        }
        new_inc_id
    }

    fn spawn_exit_thread(
        &self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        exit_rx: MpscReceiver<bool>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            exit_when_asked(mqtt_client, exit_rx);
        })
    }

    pub fn publish_incident(&self, incident: Incident, mqtt_client: &Arc<Mutex<MQTTClient>>) {
        println!("Sistema-Monitoreo: Publicando incidente.");

        // Hago el publish
        if let Ok(mut mqtt_client) = mqtt_client.lock() {
            let res_publish = mqtt_client.mqtt_publish("Inc", &incident.to_bytes());
            match res_publish {
                Ok(publish_message) => {
                    self.logger_tx
                        .send(StructsToSaveInLogger::MessageType(
                            "Sistema Monitoreo".to_string(),
                            MessageType::Publish(publish_message),
                            OperationType::Sent,
                        ))
                        .unwrap();
                }
                Err(e) => {
                    println!("Sistema-Monitoreo: Error al hacer el publish {:?}", e)
                }
            };
        }
    }
}

fn spawn_write_incidents_to_logger_thread(logger: Logger) -> JoinHandle<()> {
    thread::spawn(move || loop {
        while let Ok(msg) = logger.logger_rx.recv() {
            logger.write_in_file(msg);
        }
    })
}



