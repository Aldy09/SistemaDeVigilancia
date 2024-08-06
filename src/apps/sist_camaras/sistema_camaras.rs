use crate::apps::{
    apps_mqtt_topics::AppsMqttTopics,
    common_clients::{exit_when_asked, there_are_no_more_publish_msgs},
    incident_data::incident::{self, Incident},
    sist_camaras::{
        hashmap_incs_type::HashmapIncsType, logic::CamerasLogic,
        ai_detection::ai_detector_manager::AIDetectorManager, camera::Camera,
        shareable_cameras_type::ShCamerasType, sist_camaras_abm::ABMCameras,
    },
};
use crate::logging::string_logger::StringLogger;
use crate::mqtt::{client::mqtt_client::MQTTClient, messages::publish_message::PublishMessage};

use std::collections::HashMap;
use std::{
    fs,
    io::{self, Error, ErrorKind}, sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    }, thread::{self, JoinHandle},
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

        // ABM
        children.push(self.spawn_abm_cameras_thread(
            &self.cameras,
            self.cameras_tx.clone(),
            self.exit_tx.clone(),
        ));

        // Recibe las cámaras que envía el abm y las publica por MQTT
        children.push(self.spawn_publish_to_topic_thread(mqtt_sh.clone(), cameras_rx));
        // Exit
        children.push(spawn_exit_when_asked_thread(mqtt_sh.clone(), exit_rx));

        // Incident detector (ai)
        let (incident_tx, incident_rx) = mpsc::channel::<incident::Incident>();
        children.push(self.spawn_ai_detector_thread(incident_tx)); // conexión con proveedor intelig artificial
        children.push(self.spawn_recv_and_publish_inc_thread(incident_rx, mqtt_sh.clone())); // recibe inc y publica

        // Suscribe y recibe mensajes por MQTT
        children.push(self.spawn_subscribe_to_topics_thread(mqtt_sh.clone(), publish_msg_rx, ));

        children
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
                println!("Se recibe por rx para publicar el inc: {:?}", inc);
                if let Ok(mut mqtt_client_lock) = mqtt_client.lock() {
                    let res_publish = mqtt_client_lock.mqtt_publish(
                        AppsMqttTopics::IncidentTopic.to_str(),
                        &inc.to_bytes(),
                        qos,
                    );
                    match res_publish {
                        Ok(publish_message) => {
                            logger_thread.log(format!(
                                "Sistema-Camaras: publico inc: {:?}",
                                publish_message
                            ));
                        }
                        Err(e) => {
                            println!("Sistema-Camara: Error al hacer el publish {:?}", e);
                            logger_thread.log(format!(
                                "Sistema-Camaras: Error al hacer el publish {:?}",
                                e
                            ));
                        }
                    };
                }
            }
        })
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

    /// Recorre las cámaras y envía cada una por el channel, para que quien lea del rx haga el publish.
    fn send_cameras_from_file_to_publish(&self) {
        match self.cameras.lock() {
            Ok(cams) => {
                for camera in (*cams).values() {
                    println!("CÁMARAS: iniciando, enviando cámara: {:?}", camera);
                    self.send_camera_bytes(camera, &self.cameras_tx);
                }
            }
            Err(_) => println!("Error al tomar lock de cámaras."),
        }
    }

    /// Envía la cámara recibida, por el channel, para que quien la reciba por rx haga el publish.
    /// Además logguea la operación.
    fn send_camera_bytes(&self, camera: &Camera, camera_tx: &Sender<Vec<u8>>) {
        self.logger
            .log(format!("Sistema-Camaras: envío cámara: {:?}", camera));

        if camera_tx.send(camera.to_bytes()).is_err() {
            println!("Error al enviar cámara por tx desde hilo abm.");
            self.logger
                .log("Sistema-Camaras: error al enviar cámara por tx desde hilo abm.".to_string());
        }
    }

    /// Envía todas las cámaras al otro hilo para ser publicadas.
    /// Y lanza el hilo encargado de ejecutar el abm.
    fn spawn_abm_cameras_thread(
        &self,
        cameras: &Arc<Mutex<HashMap<u8, Camera>>>,
        cameras_tx: Sender<Vec<u8>>,
        exit_tx: Sender<bool>,
    ) -> JoinHandle<()> {
        // Publica cámaras al inicio
        self.send_cameras_from_file_to_publish();
        // Lanza el hilo para el abm
        let cameras_c = cameras.clone();
        let logger_c = self.logger.clone_ref();
        thread::spawn(move || {
            // Ejecuta el menú del abm
            let mut abm_cameras = ABMCameras::new(cameras_c, cameras_tx, exit_tx, logger_c);
            abm_cameras.run();
        })
    }

    fn subscribe_to_topics(
        &self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        topics: Vec<String>,
    ) -> Result<(), Error> {
        let topics_to_log = topics.to_vec();
        if let Ok(mut mqtt_client_lock) = mqtt_client.lock() {
            let res_subscribe = mqtt_client_lock.mqtt_subscribe(topics);
            match res_subscribe {
                Ok(_) => {
                    self.logger.log(format!(
                        "Sistema-Camaras: subscripto a topic: {:?}",
                        topics_to_log
                    ));
                }
                Err(e) => {
                    println!("Sistema-Camara: Error al subscribirse {:?}", e);
                    self.logger
                        .log(format!("Sistema-Camaras: Error al subscribirse: {:?}", e));
                    return Err(e);
                }
            };
        }
        Ok(())
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
                    Ok(publish_message) => {
                        self.logger.log(format!(
                            "Sistema-Camaras: envió mensaje: {:?}",
                            publish_message
                        ));
                    }
                    Err(e) => {
                        println!("Sistema-Camara: Error al hacer el publish {:?}", e);
                        self.logger.log(format!(
                            "Sistema-Camaras: Error al hacer el publish {:?}",
                            e
                        ));
                    }
                };
            }
        }
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

    /*fn handle_received_message(
        &mut self,
        msg: PublishMessage,
        cameras: &mut ShCamerasType,
        incs_being_managed: &mut HashmapIncsType,
        logic: CamerasLogic,
    ) {
        if let Ok(incident) = Incident::from_bytes(msg.get_payload()) {
            self.logger.log(format!(
                "Sistema-Camaras: recibió incidente: {:?}",
                incident
            ));
            self.manage_incidents(incident, cameras, incs_being_managed);
        }
    }*/

    /// Recibe mensajes de los topics a los que se ha suscrito.
    fn receive_messages_from_subscribed_topics(
        &mut self,
        rx: Receiver<PublishMessage>,
        cameras: &mut ShCamerasType,
    ) {
        let mut incs_being_managed: HashmapIncsType = HashMap::new(); // se borrará []
        let mut logic = CamerasLogic::new(cameras.clone(), self.cameras_tx.clone(), self.logger.clone_ref());

        for msg in rx {
            //self.handle_received_message(msg, cameras, &mut incs_being_managed.clone());
            if let Ok(incident) = Incident::from_bytes(msg.get_payload()) {
                self.logger
                    .log(format!("Sistema-Camaras: recibió inc: {:?}", incident));
                logic.manage_incidents(incident, cameras, &mut incs_being_managed);
            }
        }

        there_are_no_more_publish_msgs(&self.logger);
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
            let res =
                self_clone.subscribe_to_topics(mqtt_client.clone(), vec![String::from(topic)]);
            match res {
                Ok(_) => {
                    println!("Sistema-Camara: Subscripción a exitosa");
                    self_clone.receive_messages_from_subscribed_topics(msg_rx, &mut cameras_cloned,);
                }
                Err(e) => println!("Sistema-Camara: Error al subscribirse {:?}", e),
            };
            println!("Saliendo del hilo que recibe los PublishMessage's.");
        })
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
