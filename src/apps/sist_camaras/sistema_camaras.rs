use crate::apps::{
    apps_mqtt_topics::AppsMqttTopics,
    common_clients::{exit_when_asked, there_are_no_more_publish_msgs},
    incident_data::incident::{self, Incident},
    sist_camaras::{
        ai_detection::ai_detector_manager::AIDetectorManager, camera::Camera,
        logic::CamerasLogic,
        sist_camaras_abm::ABMCameras,
        types::shareable_cameras_type::ShCamerasType
    },
};
use crate::logging::string_logger::StringLogger;
use crate::mqtt::{client::mqtt_client::MQTTClient, messages::publish_message::PublishMessage};

use std::collections::HashMap;
use std::{
    fs,
    io::{self, ErrorKind},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

#[derive(Debug)]
pub struct SistemaCamaras {
    cameras_tx: Sender<Vec<u8>>,
    exit_tx: Sender<bool>,
    cameras: Arc<Mutex<HashMap<u8, Camera>>>,
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
    Ok(valor_qos)
}
impl SistemaCamaras {
    pub fn new(
        cameras_tx: Sender<Vec<u8>>,
        exit_tx: Sender<bool>,
        cameras: Arc<Mutex<HashMap<u8, Camera>>>,
        logger: StringLogger,
    ) -> Self {
        println!("Sistema de Cámaras\n");
        let qos =
            leer_qos_desde_archivo("src/apps/sist_camaras/qos_sistema_camaras.properties").unwrap();

        let sistema_camaras: SistemaCamaras = Self {
            cameras_tx,
            exit_tx,
            cameras,
            qos,
            logger,
        };

        sistema_camaras
    }

    pub fn spawn_threads(
        &mut self,
        cameras_rx: Receiver<Vec<u8>>,
        exit_rx: Receiver<bool>,
        publish_msg_rx: Receiver<PublishMessage>,
        mqtt_client: MQTTClient,
    ) -> Vec<JoinHandle<()>> {
        let mut children: Vec<JoinHandle<()>> = vec![];

        let mqtt_sh = Arc::new(Mutex::new(mqtt_client));

        // Recibe las cámaras que envía el abm y las publica por MQTT
        children.push(self.spawn_publish_to_topic_thread(mqtt_sh.clone(), cameras_rx));

        // ABM
        children.push(self.spawn_abm_cameras_thread(
            &self.cameras,
            self.cameras_tx.clone(),
            self.exit_tx.clone(),
        ));

        // Exit
        children.push(spawn_exit_when_asked_thread(mqtt_sh.clone(), exit_rx));

        // Incident detector (ai)
        let (incident_tx, incident_rx) = mpsc::channel::<incident::Incident>();
        children.push(self.spawn_ai_detector_thread(incident_tx)); // conexión con proveedor intelig artificial
        children.push(self.spawn_recv_and_publish_inc_thread(incident_rx, mqtt_sh.clone())); // recibe inc y publica

        // Suscribe y recibe mensajes por MQTT
        children.push(self.spawn_subscribe_to_topics_thread(mqtt_sh.clone(), publish_msg_rx));

        children
    }

    fn spawn_publish_to_topic_thread(
        &self,
        mqtt_client_sh: Arc<Mutex<MQTTClient>>,
        cameras_rx: Receiver<Vec<u8>>,
    ) -> JoinHandle<()> {
        let self_clone = self.clone_ref();
        thread::spawn(move || {
            self_clone.publish_to_topic(
                mqtt_client_sh,
                AppsMqttTopics::CameraTopic.to_str(),
                cameras_rx,
            );
        })
    }

    /// Envía todas las cámaras por tx para que la parte que las reciba las publique por MQTT.
    /// Y lanza el hilo encargado de ejecutar el abm.
    fn spawn_abm_cameras_thread(
        &self,
        cameras: &Arc<Mutex<HashMap<u8, Camera>>>,
        cameras_tx: Sender<Vec<u8>>,
        exit_tx: Sender<bool>,
    ) -> JoinHandle<()> {
        // Lanza el hilo para el abm
        let cameras_c = cameras.clone();
        let logger_c = self.logger.clone_ref();
        thread::spawn(move || {
            // Ejecuta el abm
            let mut abm_cameras = ABMCameras::new(cameras_c, cameras_tx, exit_tx, logger_c);
            abm_cameras.run();
        })
    }

    /// Pone en ejecución el módulo de detección automática de incidentes.
    fn spawn_ai_detector_thread(&self, tx: Sender<Incident>) -> JoinHandle<()> {
        let cameras_ref = Arc::clone(&self.cameras);
        let logger_ai = self.logger.clone_ref();
        thread::spawn(move || {
            let ai_inc_detector = AIDetectorManager::new(cameras_ref, tx, logger_ai.clone_ref());
            if ai_inc_detector.run().is_err() {
                println!("Error en detector, desde hilo para detector.");
                logger_ai.log("Error en detector, desde hilo para detector.".to_string());
            }
        })
    }

    /// Recibe los incidentes que envía el detector, y los publica por MQTT al topic de incidentes.
    fn spawn_recv_and_publish_inc_thread(
        &self,
        rx: Receiver<Incident>,
        mqtt_client: Arc<Mutex<MQTTClient>>,
    ) -> JoinHandle<()> {
        let qos = self.qos;
        let logger_thread = self.logger.clone_ref();
        thread::spawn(move || {
            for inc in rx {
                if let Ok(mut mqtt_client_lock) = mqtt_client.lock() {
                    let res_publish = mqtt_client_lock.mqtt_publish(
                        AppsMqttTopics::IncidentTopic.to_str(),
                        &inc.to_bytes(),
                        qos,
                    );
                    match res_publish {
                        Ok(publish_message) => {
                            logger_thread.log(format!("Publico inc: {:?}", publish_message));
                        }
                        Err(e) => {
                            println!("Error al hacer el publish {:?}", e);
                            logger_thread.log(format!("Error al hacer el publish {:?}", e));
                        }
                    };
                }
            }
        })
    }

    fn subscribe_to_topics(
        &self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        topics: Vec<String>,
    ) {
        let topics_log = topics.to_vec();
        if let Ok(mut mqtt_client_lock) = mqtt_client.lock() {
            let res_subscribe = mqtt_client_lock.mqtt_subscribe(topics);
            match res_subscribe {
                Ok(_) => {
                    self.logger
                        .log(format!("Subscripto a topic: {:?}", topics_log));
                }
                Err(e) => {
                    self.logger.log(format!("Error al subscribirse: {:?}", e));
                }
            };
        }
    }

    fn publish_to_topic(
        &self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        topic: &str,
        rx: Receiver<Vec<u8>>,
    ) {
        while let Ok(cam_bytes) = rx.recv() {
            if let Ok(mut mqtt_client_lock) = mqtt_client.lock() {
                let res_publish = mqtt_client_lock.mqtt_publish(topic, &cam_bytes, self.qos);
                match res_publish {
                    Ok(publish_msg) => {
                        self.logger.log(format!("Enviado msj: {:?}", publish_msg));
                    }
                    Err(e) => {
                        println!("Error al hacer publish {:?}", e);
                        self.logger.log(format!("Error al hacer publish {:?}", e));
                    }
                };
            }
        }
    }

    fn spawn_subscribe_to_topics_thread(
        &mut self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        msg_rx: Receiver<PublishMessage>,
    ) -> JoinHandle<()> {
        let mut cameras_cloned = self.cameras.clone();
        let mut self_clone = self.clone_ref();
        let topic = AppsMqttTopics::IncidentTopic.to_str();
        thread::spawn(move || {
            self_clone.subscribe_to_topics(mqtt_client.clone(), vec![String::from(topic)]); 
            self_clone.receive_messages_from_subscribed_topics(msg_rx, &mut cameras_cloned);
        })
    }

    /// Recibe mensajes de los topics a los que se ha suscrito, y delega el procesamiento a `CamerasLogic`.
    fn receive_messages_from_subscribed_topics(
        &mut self,
        rx: Receiver<PublishMessage>,
        cameras: &mut ShCamerasType,
    ) {
        let mut logic = CamerasLogic::new(
            cameras.clone(),
            self.cameras_tx.clone(),
            self.logger.clone_ref(),
        );

        for msg in rx {
            if let Ok(incident) = Incident::from_bytes(msg.get_payload()) {
                self.logger.log(format!("Inc recibido: {:?}", incident));
                logic.manage_incident(incident);
            }
        }

        there_are_no_more_publish_msgs(&self.logger);
    }

    fn clone_ref(&self) -> Self {
        Self {
            cameras_tx: self.cameras_tx.clone(),
            exit_tx: self.exit_tx.clone(),
            cameras: self.cameras.clone(),
            qos: self.qos,
            logger: self.logger.clone_ref(),
        }
    }
}

fn spawn_exit_when_asked_thread(
    mqtt_client_sh: Arc<Mutex<MQTTClient>>,
    exit_rx: Receiver<bool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        exit_when_asked(mqtt_client_sh, exit_rx);
    })
}
