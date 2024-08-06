use std::sync::mpsc::Receiver;

use crate::apps::{
    apps_mqtt_topics::AppsMqttTopics,
    common_clients::there_are_no_more_publish_msgs,
    sist_camaras::{
        ai_detection::ai_detector_manager::AIDetectorManager, camera::Camera,
        shareable_cameras_type::ShCamerasType, sist_camaras_abm::ABMCameras,
    },
};
use crate::apps::{
    common_clients::exit_when_asked,
    incident_data::{
        incident::{self, Incident},
        incident_info::IncidentInfo,
    },
};
use crate::logging::string_logger::StringLogger;
use crate::mqtt::{client::mqtt_client::MQTTClient, messages::publish_message::PublishMessage};

// Importa la estructura desde el módulo
use std::collections::HashMap;
use std::{
    fs,
    io::{self, Error, ErrorKind},
};
use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex, MutexGuard,
    },
    thread::{self, JoinHandle},
};

use super::{hashmap_incs_type::HashmapIncsType, logic::CamerasLogic};

#[derive(Debug)]
pub struct SistemaCamaras {
    cameras_tx: mpsc::Sender<Vec<u8>>,
    exit_tx: mpsc::Sender<bool>,
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
        children.push(self.spawn_subscribe_to_topics_thread(mqtt_sh.clone(), publish_msg_rx));

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
        cameras_tx: mpsc::Sender<Vec<u8>>,
        exit_tx: mpsc::Sender<bool>,
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
        cameras_rx: mpsc::Receiver<Vec<u8>>,
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
    fn spawn_ai_detector_thread(&self, tx: mpsc::Sender<Incident>) -> JoinHandle<()> {
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

    fn handle_received_message(
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
    }

    /// Procesa un Incidente recibido.
    fn manage_incidents(
        &mut self,
        incident: Incident,
        cameras: &mut ShCamerasType,
        incs_being_managed: &mut HashmapIncsType,
    ) {
        // Proceso los incidentes
        if !incs_being_managed.contains_key(&incident.get_info()) {
            self.process_first_time_incident(cameras, incident, incs_being_managed);
        } else {
            self.process_known_incident(cameras, incident, incs_being_managed);
        }
    }

    // Aux: (condición "hasta que" del enunciado).
    /// Procesa un incidente cuando un incidente con ese mismo id ya fue recibido anteriormente.
    /// Si su estado es resuelto, vuelve el estado de la/s cámara/s que lo atendían, a ahorro de energía.
    fn process_known_incident(
        &self,
        cameras: &mut ShCamerasType,
        inc: Incident,
        incs_being_managed: &mut HashmapIncsType,
    ) {
        if inc.is_resolved() {
            println!(
                "Recibo el incidente {} de nuevo, y ahora viene con estado resuelto.",
                inc.get_id()
            );
            self.logger.log(format!(
                "Recibo el incidente {} de nuevo, y ahora viene con estado resuelto.",
                inc.get_id()
            ));
            // Busco la/s cámara/s que atendían este incidente
            if let Some(cams_managing_inc) = incs_being_managed.get(&inc.get_info()) {
                // sé que existe, por el if de más arriba

                // Cambio el estado de las cámaras que lo manejaban, otra vez a ahorro de energía
                // solamente si el incidente en cuestión era el único que manejaban (si tenía más incidentes en rango, sigue estando activa)
                for camera_id in cams_managing_inc {
                    match cameras.lock() {
                        Ok(mut cams) => {
                            // Actualizo las cámaras en cuestión
                            if let Some(camera_to_update) = cams.get_mut(camera_id) {
                                let state_has_changed =
                                    camera_to_update.remove_from_incs_being_managed(inc.get_info());
                                self.logger.log(format!(
                                    "  la cámara queda: cam id y lista de incs: {:?}",
                                    camera_to_update.get_id_and_incs_for_debug_display()
                                ));
                                if state_has_changed {
                                    self.logger.log(format!(
                                        "Cambiado estado a ActiveMode, enviando cámara: {:?}",
                                        camera_to_update
                                    ));
                                    self.send_camera_bytes(camera_to_update, &self.cameras_tx);
                                }
                            }
                        }
                        Err(_) => println!(
                            "Error al tomar lock de cámaras para volver estado a ahorro energía."
                        ),
                    };
                }
            }
            // También elimino la entrada del hashmap que busca por incidente, ya no le doy seguimiento
            incs_being_managed.remove(&inc.get_info());
        }
    }

    /// Procesa un incidente cuando el mismo fue recibido por primera vez.
    /// Para cada cámara ve si inc.pos está dentro de alcance de dicha cámara o sus lindantes,
    /// en caso afirmativo, se encarga de lo necesario para que la cámara y sus lindanes cambien su estado a activo.
    fn process_first_time_incident(
        &self,
        cameras: &mut ShCamerasType,
        inc: Incident,
        incs_being_managed: &mut HashmapIncsType,
    ) {
        match cameras.lock() {
            Ok(mut cams) => {
                println!("Proceso el incidente {:?} por primera vez", inc.get_info());
                self.logger.log(format!(
                    "Proceso el incidente {:?} por primera vez",
                    inc.get_info()
                ));
                let cameras_that_follow_inc =
                    self.get_id_of_cameras_that_will_change_state_to_active(&mut cams, &inc);

                // El vector tiene los ids de todas las cámaras que deben cambiar a activo
                for cam_id in &cameras_that_follow_inc {
                    if let Some(bordering_cam) = cams.get_mut(cam_id) {
                        // Agrega el inc a la lista de incs de la cámara, y de sus lindantes, para facilitar que luego puedan volver a su anterior estado
                        let state_has_changed =
                            bordering_cam.append_to_incs_being_managed(inc.get_info());
                        if state_has_changed {
                            self.logger.log(format!(
                                "Cambiando a SavingMode, enviando cámara: {:?}",
                                bordering_cam
                            ));
                            self.send_camera_bytes(bordering_cam, &self.cameras_tx);
                        }
                    };
                }
                // Y se guarda las cámaras que le dan seguimiento al incidente, para luego poder encontrarlas fácilmente sin recorrer
                incs_being_managed.insert(inc.get_info(), cameras_that_follow_inc);
            }
            Err(_) => todo!(),
        }
    }

    /// Devuelve un vector de u8 con los ids de todas las cámaras que darán seguimiento al incidente `inc`.
    fn get_id_of_cameras_that_will_change_state_to_active(
        &self,
        cams: &mut MutexGuard<'_, HashMap<u8, Camera>>,
        inc: &Incident,
    ) -> Vec<u8> {
        let mut cameras_that_follow_inc = vec![];

        // Recorremos cada una de las cámaras, para ver si el inc está en su rango
        for (cam_id, camera) in cams.iter_mut() {
            if camera.will_register(inc.get_position()) {
                self.logger.log(format!(
                    "Está en rango de cam: {}, cambiando su estado a activo.",
                    cam_id
                ));

                cameras_that_follow_inc.push(*cam_id);

                for bordering_cam_id in camera.get_bordering_cams() {
                    cameras_that_follow_inc.push(*bordering_cam_id);
                }
                self.logger.log(format!(
                    "  la cámara queda: cam id y lista de incs: {:?}",
                    camera.get_id_and_incs_for_debug_display()
                ));
            }
        }

        cameras_that_follow_inc
    }

    /// Recibe mensajes de los topics a los que se ha suscrito.
    fn receive_messages_from_subscribed_topics(
        &mut self,
        rx: Receiver<PublishMessage>,
        cameras: &mut ShCamerasType,
    ) {
        let mut incs_being_managed: HashmapIncsType = HashMap::new(); // se borrará []
        let logic = CamerasLogic::new(cameras.clone());

        for msg in rx {
            //self.handle_received_message(msg, cameras, &mut incs_being_managed.clone());
            if let Ok(incident) = Incident::from_bytes(msg.get_payload()) {
                self.logger
                    .log(format!("Sistema-Camaras: recibió inc: {:?}", incident));
                self.manage_incidents(incident, cameras, &mut incs_being_managed);
            }
        }

        there_are_no_more_publish_msgs(&self.logger);
    }

    fn spawn_subscribe_to_topics_thread(
        &mut self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        rx: Receiver<PublishMessage>,
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
                    self_clone.receive_messages_from_subscribed_topics(rx, &mut cameras_cloned);
                }
                Err(e) => println!("Sistema-Camara: Error al subscribirse {:?}", e),
            };
            println!("Saliendo del hilo que recibe los PublishMessage's.");
        })
    }
}

fn spawn_exit_when_asked_thread(
    mqtt_client_sh: Arc<Mutex<MQTTClient>>,
    exit_rx: mpsc::Receiver<bool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        exit_when_asked(mqtt_client_sh, exit_rx);
    })
}
