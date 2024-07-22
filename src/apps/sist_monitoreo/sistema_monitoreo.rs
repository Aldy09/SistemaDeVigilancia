use std::{
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

use crossbeam_channel::Receiver as CrossbeamReceiver;
use crossbeam_channel::Sender as CrossbeamSender;

use std::sync::mpsc::{Receiver as MpscReceiver, Sender as MpscSender};

use crate::{
    apps::{apps_mqtt_topics::AppsMqttTopics, common_clients::is_disconnected_error},
    logging::{
        logger::Logger, string_logger::StringLogger, structs_to_save_in_logger::{OperationType, StructsToSaveInLogger}
    },
};

use crate::mqtt::{
    client::mqtt_client::MQTTClient,
    messages::{message_type::MessageType, publish_message::PublishMessage},
};

use super::ui_sistema_monitoreo::UISistemaMonitoreo;
use crate::apps::{app_type::AppType, common_clients::exit_when_asked, incident_data::incident::Incident};
use std::fs;
use std::io::{self, ErrorKind};

#[derive(Debug)]
pub struct SistemaMonitoreo {
    incidents: Arc<Mutex<Vec<Incident>>>,
    logger_tx: MpscSender<StructsToSaveInLogger>,
    egui_tx: CrossbeamSender<PublishMessage>,
    qos: u8,
    logger: StringLogger,
}

fn leer_qos_desde_archivo(ruta_archivo: &str) -> Result<u8, io::Error> {
    let contenido = fs::read_to_string(ruta_archivo)?;
    let inicio = contenido.find("qos=").ok_or(io::Error::new(
        ErrorKind::NotFound,
        "No se encontró la etiqueta 'qos='",
    ))?;

    let valor_qos = contenido[inicio + 4..].trim().parse::<u8>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "El valor de QoS no es un número válido",
        )
    })?;
    println!("Valor de QoS: {}", valor_qos);
    Ok(valor_qos)
}

impl SistemaMonitoreo {
    pub fn new(
        logger_tx: MpscSender<StructsToSaveInLogger>,
        egui_tx: CrossbeamSender<PublishMessage>,
        logger: StringLogger,
    ) -> Self {
        let qos =
            leer_qos_desde_archivo("src/apps/sist_monitoreo/qos_sistema_monitoreo.properties")
                .unwrap_or(0);
        println!("valor de QoS: {}", qos);
        let sistema_monitoreo: SistemaMonitoreo = Self {
            incidents: Arc::new(Mutex::new(Vec::new())),
            logger_tx,
            egui_tx,
            qos,
            logger,
        };

        sistema_monitoreo
    }

    pub fn spawn_threads(
        &self,
        logger_rx: MpscReceiver<StructsToSaveInLogger>,
        publish_message_rx: MpscReceiver<PublishMessage>,
        egui_rx: CrossbeamReceiver<PublishMessage>,
        mqtt_client: MQTTClient,
    ) -> Vec<JoinHandle<()>> {
        let (incident_tx, incident_rx) = mpsc::channel::<Incident>();
        let (exit_tx, exit_rx) = mpsc::channel::<bool>();

        let old_logger = Logger::new(logger_rx);
        let mut children: Vec<JoinHandle<()>> = vec![];
        let mqtt_client_sh = Arc::new(Mutex::new(mqtt_client));
        let mqtt_client_incident_sh_clone = Arc::clone(&mqtt_client_sh.clone());

        children.push(
            self.spawn_subscribe_to_topics_thread(mqtt_client_sh.clone(), publish_message_rx),
        );

        // Thread para hacer publish de un incidente que llega atraves de la UI
        children.push(self.spawn_publish_incidents_in_topic_thread(
            mqtt_client_incident_sh_clone.clone(),
            incident_rx,
        ));

        // AUX: Esta función se está reemplazando por el StringLogger. Una vez terminado el refactor, borrar esta función. [].
        children.push(spawn_write_incidents_to_logger_thread(old_logger));

        children.push(self.spawn_exit_thread(mqtt_client_incident_sh_clone.clone(), exit_rx));

        self.spawn_ui_thread(incident_tx, egui_rx, exit_tx);

        children
    }
    pub fn get_qos(&self) -> u8 {
        self.qos
    }

    fn spawn_ui_thread(
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

    fn spawn_publish_incidents_in_topic_thread(
        &self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        rx: MpscReceiver<Incident>,
    ) -> JoinHandle<()> {
        let self_clone = self.clone_ref();
        thread::spawn(move || loop {
            while let Ok(inc) = rx.recv() {
                let msg_clone = inc.clone();
                // AUX: esta línea de logger_tx, se puede reemplazar por el logger.log de abajo (que no usa structs). []
                self_clone
                    .logger_tx
                    .send(StructsToSaveInLogger::AppType(
                        "Sistema-Monitoreo".to_string(),
                        AppType::Incident(inc.clone()),
                        OperationType::Sent,
                    ))
                    .unwrap();
                
                self_clone.logger.log(format!("Sistema-Monitoreo: envió incidente: {:?}", inc));
                self_clone.publish_incident(msg_clone, &mqtt_client);
            }
        })
    }

    fn clone_ref(&self) -> Self {
        Self {
            incidents: self.incidents.clone(),
            egui_tx: self.egui_tx.clone(),
            logger_tx: self.logger_tx.clone(),
            qos: self.qos,
            logger: self.logger.clone_ref(),
        }
    }

    fn spawn_subscribe_to_topics_thread(
        &self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        mqtt_rx: MpscReceiver<PublishMessage>,
    ) -> JoinHandle<()> {
        let self_clone = self.clone_ref();
        thread::spawn(move || {
            self_clone.subscribe_to_topics(mqtt_client, mqtt_rx);
        })
    }

    /// Se suscribe a los topics y queda recibiendo PublishMessages de esos topics.
    fn subscribe_to_topics(
        &self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        mqtt_rx: MpscReceiver<PublishMessage>,
    ) {
        self.subscribe_to_topic(&mqtt_client, AppsMqttTopics::CameraTopic.to_str());
        self.subscribe_to_topic(&mqtt_client, AppsMqttTopics::DronTopic.to_str());
        self.subscribe_to_topic(&mqtt_client, AppsMqttTopics::IncidentTopic.to_str());
        self.receive_messages_from_subscribed_topics(mqtt_rx);
    }

    fn subscribe_to_topic(&self, mqtt_client: &Arc<Mutex<MQTTClient>>, topic: &str) {
        if let Ok(mut mqtt_client) = mqtt_client.lock() {
            let res_sub = mqtt_client.mqtt_subscribe(vec![(String::from(topic))]);
            match res_sub {
                Ok(subscribe_message) => {
                    // AUX: esta línea de logger_tx, se puede reemplazar por el logger.log de abajo (que no usa structs). []
                    self.logger_tx
                    .send(StructsToSaveInLogger::MessageType(
                        "Sistema Monitoreo".to_string(),
                        MessageType::Subscribe(subscribe_message),
                        OperationType::Sent,
                    ))
                    .unwrap();

                    //self.logger.log(format!("Sistema-Monitoreo: envió incidente: {:?}", subscribe_message)); // descomentar cuando se borre el send de arriba. [].
                }
                Err(e) => {
                    println!("Cliente: Error al hacer un subscribe a topic: {:?}", e);
                    self.logger.log(format!("Error al hacer un subscribe al topic: {:?}", e));
                },
            }
        }
    }

    // Recibe mensajes de los topics a los que se ha suscrito
    fn receive_messages_from_subscribed_topics(&self, mqtt_rx: MpscReceiver<PublishMessage>) {
        loop {
            match mqtt_rx.recv() {
                //Publish message: camera o dron
                Ok(publish_message) => {
                    // AUX: esta línea de logger_tx, se puede reemplazar por el logger.log de abajo (que no usa structs). []
                    self.logger_tx
                    .send(StructsToSaveInLogger::MessageType(
                        "Sistema Monitoreo".to_string(),
                        MessageType::Publish(publish_message.clone()),
                        OperationType::Received,
                    ))
                    .unwrap();
                    
                    self.logger.log(format!("Sistema-Monitoreo: recibió mensaje: {:?}", publish_message));
                    self.send_publish_message_to_ui(publish_message)
                }
                Err(_) => {
                    is_disconnected_error();
                    break;
                }
            }
        }
    }

    fn send_publish_message_to_ui(&self, msg: PublishMessage) {
        let res_send = self.egui_tx.send(msg);
        match res_send {
            Ok(_) => println!("Cliente: Enviado mensaje a la UI"),
            Err(e) => println!("Cliente: Error al enviar mensaje a la UI: {:?}", e),
        }
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

    fn publish_incident(&self, incident: Incident, mqtt_client: &Arc<Mutex<MQTTClient>>) {
        println!("Sistema-Monitoreo: Publicando incidente.");

        // Hago el publish
        if let Ok(mut mqtt_client) = mqtt_client.lock() {
            let res_publish = mqtt_client.mqtt_publish(AppsMqttTopics::IncidentTopic.to_str(), &incident.to_bytes(), self.get_qos());
            match res_publish {
                Ok(publish_message) => {
                    // AUX: esta línea de logger_tx, se puede reemplazar por el logger.log de abajo (que no usa structs). []
                    self.logger_tx
                    .send(StructsToSaveInLogger::MessageType(
                        "Sistema Monitoreo".to_string(),
                        MessageType::Publish(publish_message.clone()),
                        OperationType::Sent,
                    ))
                    .unwrap();
                    
                    self.logger.log(format!("Sistema-Monitoreo: envió mensaje: {:?}", publish_message));
            }
                Err(e) => {
                    println!("Sistema-Monitoreo: Error al hacer el publish {:?}", e)
                }
            };
        }
    }
}

// AUX: Esta función se está reemplazando por el StringLogger. Una vez terminado el refactor, borrar esta función. [].
fn spawn_write_incidents_to_logger_thread(logger: Logger) -> JoinHandle<()> {
    thread::spawn(move || loop {
        while let Ok(msg) = logger.logger_rx.recv() {
            logger.write_in_file(msg);
        }
    })
}
