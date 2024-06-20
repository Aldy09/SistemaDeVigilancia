type ShareableCamType = Camera;
type ShCamerasType = Arc<Mutex<HashMap<u8, ShareableCamType>>>;
use crate::messages::publish_message::PublishMessage;
use crate::structs_to_save_in_logger::OperationType;
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::mpsc::Receiver as MpscReceiver;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;
//use std::sync::mpsc::Sender as MpscSender;
type Channels = (
    mpsc::Sender<Vec<u8>>,
    mpsc::Receiver<Vec<u8>>,
    mpsc::Sender<bool>,
    mpsc::Receiver<bool>,
);
//use rustx::apps::properties::Properties;

//use std::env::args;
use std::io::Error;
use std::net::SocketAddr;

use crate::messages::message_type::MessageType;

use crate::structs_to_save_in_logger::StructsToSaveInLogger;
use crate::{logger::Logger, mqtt_client::MQTTClient};

use crate::apps::{
    camera::Camera,
    common_clients::{exit_when_asked, get_broker_address, join_all_threads},
    incident::Incident,
    manage_stored_cameras::read_cameras_from_file,
};

use super::app_type::AppType;

pub struct SistemaCamaras {
    cameras_tx: mpsc::Sender<Vec<u8>>,
    logger_tx: mpsc::Sender<StructsToSaveInLogger>,
    exit_tx: mpsc::Sender<bool>,
    //incs_being_managed: HashMap<u8, Vec<u8>>,
}

impl SistemaCamaras {
    pub fn new() -> Self {
        println!("SISTEMA DE CAMARAS\n");

        let broker_addr = get_broker_address();
        let (logger_tx, logger_rx) = mpsc::channel::<StructsToSaveInLogger>();
        let (cameras_tx, cameras_rx, exit_tx, exit_rx) = create_channels();
        //let incs_being_managed: HashMap<u8, Vec<u8>> = HashMap::new();

        let mut sistema_camaras: SistemaCamaras = Self {
            cameras_tx,
            logger_tx,
            exit_tx,
            //incs_being_managed,
        };

        match establish_mqtt_broker_connection(&broker_addr) {
            Ok(mqtt_client) => {
                let shareable_cameras = create_cameras();
                sleep(Duration::from_secs(2)); // Sleep to start everything 'at the same time'
                let children = sistema_camaras.spawn_threads(
                    mqtt_client,
                    shareable_cameras,
                    cameras_rx,
                    logger_rx,
                    exit_rx,
                );
                join_all_threads(children);
            }
            Err(e) => println!("Error al conectar al broker MQTT: {:?}", e),
        }

        sistema_camaras
    }

    fn spawn_threads(
        &mut self,
        mqtt_client: MQTTClient,
        shareable_cameras: Arc<Mutex<HashMap<u8, Camera>>>,
        cameras_rx: MpscReceiver<Vec<u8>>,
        logger_rx: MpscReceiver<StructsToSaveInLogger>,
        exit_rx: MpscReceiver<bool>,
    ) -> Vec<JoinHandle<()>> {
        let mut children: Vec<JoinHandle<()>> = vec![];

        let mqtt_client_sh = Arc::new(Mutex::new(mqtt_client));

        let logger = Logger::new(logger_rx);

        let self_clone = self.clone_ref();

        children.push(self.spawn_abm_cameras_thread(
            shareable_cameras.clone(),
            self_clone.cameras_tx,
            self_clone.exit_tx,
        ));
        children.push(self.spawn_publish_to_topic_thread(mqtt_client_sh.clone(), cameras_rx));
        children.push(spawn_exit_when_asked_thread(
            mqtt_client_sh.clone(),
            exit_rx,
        ));
        children.push(self.spawn_subscribe_to_topics_thread(
            mqtt_client_sh.clone(),
            &mut shareable_cameras.clone(),
        ));

        children.push(spawn_receive_messages_thread(logger));

        children
    }

    pub fn clone_ref(&self) -> Self {
        Self {
            cameras_tx: self.cameras_tx.clone(),
            logger_tx: self.logger_tx.clone(),
            exit_tx: self.exit_tx.clone(),
            //incs_being_managed: self.incs_being_managed.clone(),
        }
    }

    fn send_camera_data(&self, camera: &Camera, camera_tx: &Sender<Vec<u8>>) {
        let camera_clone = camera.clone();
        self.logger_tx
            .send(StructsToSaveInLogger::AppType(
                "Sistema Camaras".to_string(),
                AppType::Camera(camera_clone.clone()),
                OperationType::Sent,
            ))
            .unwrap();
        if camera_tx.send(camera.to_bytes()).is_err() {
            println!("Error al enviar cámara por tx desde hilo abm.");
        }
    }

    fn send_cameras_from_file_to_publish(
        &self,
        cameras: &mut ShCamerasType,
        camera_tx: &Sender<Vec<u8>>,
    ) {
        match cameras.lock() {
            Ok(cams) => {
                for camera in (*cams).values() {
                    self.send_camera_data(camera, camera_tx);
                }
            }
            Err(_) => println!("Error al tomar lock de cámaras."),
        }
    }

    pub fn abm_cameras(
        &mut self,
        cameras: &mut ShCamerasType,
        camera_tx: Sender<Vec<u8>>,
        exit_tx: Sender<bool>,
    ) {
        // Publicar cámaras al inicio
        self.send_cameras_from_file_to_publish(cameras, &camera_tx);

        loop {
            print_menu_abm();
            let input = get_input_abm(None);

            match &*input {
                "1" => {
                    let camera = create_camera_abm();
                    self.send_camera_abm(cameras, &camera_tx, camera);
                }
                "2" => show_cameras_abm(cameras),
                "3" => delete_camera_abm(cameras, &camera_tx),
                "4" => {
                    exit_program_abm(&exit_tx);
                    break;
                }
                _ => {
                    println!("Opción no válida. Intente nuevamente.\n");
                }
            }
        }
    }

    fn spawn_abm_cameras_thread(
        &self,
        shareable_cameras: Arc<Mutex<HashMap<u8, Camera>>>,
        cameras_tx: mpsc::Sender<Vec<u8>>,
        exit_tx: mpsc::Sender<bool>,
    ) -> JoinHandle<()> {
        let mut self_clone = self.clone_ref();
        thread::spawn(move || {
            self_clone.abm_cameras(&mut shareable_cameras.clone(), cameras_tx, exit_tx);
        })
    }

    fn process_camera(
        &mut self,
        cams: &mut HashMap<u8, ShareableCamType>,
        camera_clone: Camera,
        camera_tx: &Sender<Vec<u8>>,
    ) {
        // Recorre las cámaras ya existentes, agregando la nueva cámara como lindante de la que corresponda y viceversa, terminando la creación
        for camera in cams.values_mut() {
            camera.mutually_add_if_bordering(&mut camera_clone.clone());
        }
        //Envia la camara al log.txt
        self.logger_tx
            .send(StructsToSaveInLogger::AppType(
                "Sistema Camaras".to_string(),
                AppType::Camera(camera_clone.clone()),
                OperationType::Sent,
            ))
            .unwrap();
        // Envía la nueva cámara por tx, para ser publicada por el otro hilo
        if camera_tx.send(camera_clone.to_bytes()).is_err() {
            println!("Error al enviar cámara por tx desde hilo abm.");
        }
        // Guarda la nueva cámara
        cams.insert(camera_clone.get_id(), camera_clone);
        println!("Cámara agregada con éxito.\n");
    }

    pub fn send_camera_abm(
        &mut self,
        cameras: &mut ShCamerasType,
        camera_tx: &Sender<Vec<u8>>,
        new_camera: Camera,
    ) {
        let camera_clone = new_camera.clone();

        match cameras.lock() {
            Ok(mut cams) => {
                self.process_camera(&mut cams, camera_clone, camera_tx);
            }
            Err(e) => println!("Error tomando lock en agregar cámara abm, {:?}.\n", e),
        };
    }

    fn subscribe_to_topics(
        &self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        topics: Vec<String>,
    ) -> Result<(), Error> {
        if let Ok(mut mqtt_client_lock) = mqtt_client.lock() {
            let res_subscribe = mqtt_client_lock.mqtt_subscribe(topics);
            match res_subscribe {
                Ok(subscribe_message) => {
                    self.logger_tx
                        .send(StructsToSaveInLogger::MessageType(
                            "Sistema Camaras".to_string(),
                            MessageType::Subscribe(subscribe_message),
                            OperationType::Sent,
                        ))
                        .unwrap();
                }
                Err(e) => {
                    println!("Sistema-Camara: Error al subscribirse {:?}", e);
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
                let res_publish = mqtt_client_lock.mqtt_publish(topic, &cam_bytes);
                match res_publish {
                    Ok(publish_message) => {
                        self.logger_tx
                            .send(StructsToSaveInLogger::MessageType(
                                "Sistema Camaras".to_string(),
                                MessageType::Publish(publish_message),
                                OperationType::Sent,
                            ))
                            .unwrap();
                    }
                    Err(e) => println!("Sistema-Camara: Error al hacer el publish {:?}", e),
                };
            }
        }
    }

    pub fn spawn_publish_to_topic_thread(
        &self,
        mqtt_client_sh: Arc<Mutex<MQTTClient>>,
        cameras_rx: mpsc::Receiver<Vec<u8>>,
    ) -> JoinHandle<()> {
        let self_clone = self.clone_ref();
        thread::spawn(move || {
            self_clone.publish_to_topic(mqtt_client_sh, "Cam", cameras_rx);
        })
    }

    fn handle_received_message(&mut self, msg: PublishMessage, cameras: &mut ShCamerasType, incs_being_managed: &mut HashMap<u8, Vec<u8>>) {
        let incident = Incident::from_bytes(msg.get_payload());
        self.logger_tx
            .send(StructsToSaveInLogger::AppType(
                "Sistema Camaras".to_string(),
                AppType::Incident(incident.clone()),
                OperationType::Received,
            ))
            .unwrap();
        self.manage_incidents(incident, cameras, incs_being_managed);
    }

    /// Procesa un Incidente recibido.
    fn manage_incidents(&mut self, incident: Incident, cameras: &mut ShCamerasType, incs_being_managed: &mut HashMap<u8, Vec<u8>>) {
        // Proceso los incidentes
        if !incs_being_managed.contains_key(&incident.id) {
            procesar_incidente_por_primera_vez(cameras, incident, incs_being_managed);
        } else {
            procesar_incidente_conocido(cameras, incident, incs_being_managed);
        }
    }

    /// Recibe mensajes de los topics a los que se ha suscrito.
    pub fn receive_messages_from_subscribed_topics(
        &mut self,
        mqtt_client: &Arc<Mutex<MQTTClient>>,
        cameras: &mut ShCamerasType,
    ) {
        let mut incs_being_managed: HashMap<u8, Vec<u8>> = HashMap::new();
        loop {
            if let Ok(mqtt_client) = mqtt_client.lock() {
                match mqtt_client.mqtt_receive_msg_from_subs_topic() {
                    Ok(msg) => self.handle_received_message(msg, cameras, &mut incs_being_managed),
                    Err(e) => {
                        if !handle_message_receiving_error(e) {
                            break;
                        }
                    }
                }
            }
        }
    }

    fn spawn_subscribe_to_topics_thread(
        &mut self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        cameras: &mut Arc<Mutex<HashMap<u8, Camera>>>,
    ) -> JoinHandle<()> {
        let mut cameras_cloned = cameras.clone();
        let mut self_clone = self.clone_ref();
        thread::spawn(move || {
            let res = self_clone.subscribe_to_topics(mqtt_client.clone(), vec!["Inc".to_string()]);
            match res {
                Ok(_) => {
                    println!("Sistema-Camara: Subscripción a exitosa");
                    self_clone.receive_messages_from_subscribed_topics(
                        &mqtt_client.clone(),
                        &mut cameras_cloned,
                    );
                }
                Err(e) => println!("Sistema-Camara: Error al subscribirse {:?}", e),
            };
            println!("Saliendo del hilo de subscribirme");
        })
    }
}

fn spawn_receive_messages_thread(logger: Logger) -> JoinHandle<()> {
    thread::spawn(move || loop {
        while let Ok(msg) = logger.logger_rx.recv() {
            println!("RECIBI MENSAJE DE LOGGER EN SISTEMA CAMARAS: {:?}", msg);
            logger.write_in_file(msg);
        }
    })
}

pub fn show_cameras_abm(cameras: &mut ShCamerasType) {
    // Mostramos todas las cámaras
    println!("Cámaras registradas:\n");
    match cameras.lock() {
        Ok(cams) => {
            for camera in (*cams).values() {
                // Si no está marcada borrada, mostrarla
                if camera.is_not_deleted() {
                    camera.display();
                };
            }
        }
        Err(_) => println!("Error al tomar lock de cámaras."),
    }
}

pub fn delete_camera_abm(cameras: &mut ShCamerasType, camera_tx: &Sender<Vec<u8>>) {
    let id: u8 = get_input_abm(Some("Ingrese el ID de la cámara a eliminar: "))
        .parse()
        .expect("Id no válido");
    match cameras.lock() {
        Ok(mut cams) => {
            if let Some(mut camera_to_delete) = cams.remove(&id) {
                if camera_to_delete.is_not_deleted() {
                    camera_to_delete.delete_camera();

                    // Recorre las cámaras ya existentes, eliminando la cámara a eliminar como lindante de la que corresponda, terminando la eliminación
                    for camera in cams.values_mut() {
                        camera.remove_from_list_if_bordering(&mut camera_to_delete);
                    }

                    // Envía por el tx la cámara a eliminar para que se publique desde el otro hilo
                    // (con eso es suficiente. Si bien se les eliminó una lindante, no es necesario publicar el cambio
                    // de las demás ya que eso solo es relevante para sistema camaras)
                    if camera_tx.send(camera_to_delete.to_bytes()).is_err() {
                        println!("Error al enviar cámara por tx desde hilo abm.");
                    } else {
                        println!("Cámara eliminada con éxito.\n");
                    }
                };
            }
        }
        Err(e) => println!("Error tomando lock baja abm, {:?}.\n", e),
    };
}

pub fn exit_program_abm(exit_tx: &Sender<bool>) {
    match exit_tx.send(true) {
        Ok(_) => println!("Saliendo del programa."),
        Err(e) => println!("Error al intentar salir: {:?}", e),
    }
}

pub fn print_menu_abm() {
    println!(
        "      MENÚ
    1. Agregar cámara
    2. Mostrar cámaras
    3. Eliminar cámara
    4. Salir
    Ingrese una opción:"
    );
}

pub fn create_camera_abm() -> Camera {
    let id: u8 = get_input_abm(Some("Ingrese el ID de la cámara: "))
        .parse()
        .expect("ID no válido");
    let latitude: f64 = get_input_abm(Some("Ingrese Latitud: "))
        .parse()
        .expect("Latitud no válida");
    let longitude: f64 = get_input_abm(Some("Ingrese Longitud: "))
        .parse()
        .expect("Longitud no válida");
    let range: u8 = get_input_abm(Some("Ingrese el rango: "))
        .parse()
        .expect("Rango no válido");

    Camera::new(id, latitude, longitude, range)
}

pub fn get_input_abm(prompt: Option<&str>) -> String {
    if let Some(p) = prompt {
        print!("{}", p);
    }
    let _ = io::stdout().flush();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Error al leer la entrada");
    input.trim().to_string()
}

pub fn establish_mqtt_broker_connection(broker_addr: &SocketAddr) -> Result<MQTTClient, Error> {
    let client_id = "Sistema-Camaras";
    let mqtt_client_res = MQTTClient::mqtt_connect_to_broker(client_id, broker_addr);
    match mqtt_client_res {
        Ok(mqtt_client) => {
            println!("Cliente: Conectado al broker MQTT.");
            Ok(mqtt_client)
        }
        Err(e) => {
            println!("Sistema-Camara: Error al conectar al broker MQTT: {:?}", e);
            Err(e)
        }
    }
}

pub fn handle_message_receiving_error(e: std::io::Error) -> bool {
    match e.kind() {
        std::io::ErrorKind::TimedOut => true,
        std::io::ErrorKind::NotConnected => {
            println!("Cliente: No hay más PublishMessage's por leer.");
            false
        }
        _ => {
            println!("Cliente: error al leer los publish messages recibidos.");
            true
        }
    }
    /*/*if e == RecvTimeoutError::Timeout {
    }*/

    if e == RecvTimeoutError::Disconnected {
        println!("Cliente: No hay más PublishMessage's por leer.");
        break;
    }*/
}

fn create_cameras() -> Arc<Mutex<HashMap<u8, Camera>>> {
    let cameras: HashMap<u8, Camera> = read_cameras_from_file("./cameras.properties");
    Arc::new(Mutex::new(cameras))
}

fn create_channels() -> Channels {
    let (cameras_tx, cameras_rx) = mpsc::channel::<Vec<u8>>();
    let (exit_tx, exit_rx) = mpsc::channel::<bool>();
    (cameras_tx, cameras_rx, exit_tx, exit_rx)
}

fn spawn_exit_when_asked_thread(
    mqtt_client_sh: Arc<Mutex<MQTTClient>>,
    exit_rx: mpsc::Receiver<bool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        exit_when_asked(mqtt_client_sh, exit_rx);
    })
}

// Aux: (condición "hasta que" del enunciado).
/// Procesa un incidente cuando un incidente con ese mismo id ya fue recibido anteriormente.
/// Si su estado es resuelto, vuelve el estado de la/s cámara/s que lo atendían, a ahorro de energía.
fn procesar_incidente_conocido(
    cameras: &mut ShCamerasType,
    inc: Incident,
    incs_being_managed: &mut HashMap<u8, Vec<u8>>,
) {
    if inc.is_resolved() {
        println!(
            "Recibo el incidente {} de nuevo, y ahora viene con estado resuelto.",
            inc.id
        );
        // Busco la/s cámara/s que atendían este incidente
        if let Some(cams_managing_inc) = incs_being_managed.get(&inc.id) {
            // sé que existe, por el if de más arriba

            // Cambio el estado de las cámaras que lo manejaban, otra vez a ahorro de energía
            // solamente si el incidente en cuestión era el único que manejaban (si tenía más incidentes en rango, sigue estando activa)
            for camera_id in cams_managing_inc {
                match cameras.lock() {
                    Ok(mut cams) => {
                        // Actualizo las cámaras en cuestión
                        if let Some(camera_to_update) = cams.get_mut(camera_id) {
                            camera_to_update.remove_from_incs_being_managed(inc.id);
                            println!(
                                "  la cámara queda:\n   cam id y lista de incs: {:?}",
                                camera_to_update.get_id_e_incs_for_debug_display()
                            );
                        }
                    }
                    Err(_) => println!(
                        "Error al tomar lock de cámaras para volver estado a ahorro energía."
                    ),
                };
            }
        }
        // También elimino la entrada del hashmap que busca por incidente, ya no le doy seguimiento
        incs_being_managed.remove(&inc.id);
    }
}

/// Procesa un incidente cuando el mismo fue recibido por primera vez.
/// Para cada cámara ve si inc.pos está dentro de alcance de dicha cámara o sus lindantes,
/// en caso afirmativo, se encarga de lo necesario para que la cámara y sus lindanes cambien su estado a activo.
fn procesar_incidente_por_primera_vez(
    cameras: &mut ShCamerasType,
    inc: Incident,
    incs_being_managed: &mut HashMap<u8, Vec<u8>>,
) {
    match cameras.lock() {
        Ok(mut cams) => {
            println!("Proceso el incidente {} por primera vez", inc.id);
            let cameras_that_follow_inc =
                get_id_of_cameras_that_will_change_state_to_active(&mut cams, &inc);

            // El vector tiene los ids de todas las cámaras que deben cambiar a activo
            for cam_id in &cameras_that_follow_inc {
                if let Some(bordering_cam) = cams.get_mut(cam_id) {
                    // Agrega el inc a la lista de incs de la cámara, y de sus lindantes, para facilitar que luego puedan volver a su anterior estado
                    bordering_cam.append_to_incs_being_managed(inc.id);
                };
            }
            // Y se guarda las cámaras que le dan seguimiento al incidente, para luego poder encontrarlas fácilmente sin recorrer
            incs_being_managed.insert(inc.id, cameras_that_follow_inc);
        }
        Err(_) => todo!(),
    }
}

/// Devuelve un vector de u8 con los ids de todas las cámaras que darán seguimiento al incidente.
fn get_id_of_cameras_that_will_change_state_to_active(
    //cameras: &mut ShCamerasType,
    cams: &mut MutexGuard<'_, HashMap<u8, Camera>>,
    inc: &Incident,
) -> Vec<u8> {
    let mut cameras_that_follow_inc = vec![];

    // Recorremos cada una de las cámaras, para ver si el inc está en su rango
    for (cam_id, camera) in cams.iter_mut() {
        if camera.will_register(inc.pos()) {
            println!(
                "Está en rango de cam: {}, cambiando su estado a activo.",
                cam_id
            ); // [] ver lindantes
               // Agrega el inc a la lista de incs de la cámara, y de sus lindantes, para que luego puedan volver a su anterior estado
               //camera.append_to_incs_being_managed(inc.id);
               //let mut cameras_that_follow_inc = vec![*cam_id];
            cameras_that_follow_inc.push(*cam_id);

            for bordering_cam_id in camera.get_bordering_cams() {
                /*if let Some(bordering_cam) = cams.get_mut(&bordering_cam_id){ // <--- Aux: quiero esto, pero no me deja xq 2 veces mut :( (ni siquiera me deja con get sin mut).
                    bordering_cam.append_to_incs_being_managed(inc.id); // <-- falta esta línea, que no se puede xq 2 veces mut
                    cameras_that_follow_inc.push(*bordering_cam_id);
                };*/
                cameras_that_follow_inc.push(*bordering_cam_id); // Aux: quizás haya que pensar otro diseño, xq si no puedo hacer el bloque comentado de acá arriba se complica.
            }
            /*// Y se guarda las cámaras que le dan seguimiento al incidente, para luego poder encontrarlas fácilmente sin recorrer
            incs_being_managed.insert(inc.id, cameras_that_follow_inc);*/
            println!(
                "  la cámara queda:\n   cam id y lista de incs: {:?}",
                camera.get_id_e_incs_for_debug_display()
            );
        }
    }

    cameras_that_follow_inc
}

impl Default for SistemaCamaras {
    fn default() -> Self {
        Self::new()
    }
}
