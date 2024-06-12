use rustx::apps::camera::Camera;
use rustx::apps::common_clients::{exit_when_asked, get_broker_address, join_all_threads};
use rustx::apps::manage_stored_cameras::read_cameras_from_file;
use rustx::mqtt_client::MQTTClient;
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;
use std::sync::{mpsc, Arc};
use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;
type ShareableCamType = Camera;
type ShCamerasType = Arc<Mutex<HashMap<u8, ShareableCamType>>>;
use rustx::apps::incident::Incident;
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

fn publish_to_topic(mqtt_client: Arc<Mutex<MQTTClient>>, topic: &str, rx: Receiver<Vec<u8>>) {
    while let Ok(cam_bytes) = rx.recv() {
        if let Ok(mut mqtt_client_lock) = mqtt_client.lock() {
            let res = mqtt_client_lock.mqtt_publish(topic, &cam_bytes);
            match res {
                Ok(_) => {
                    println!("Sistema-Camara: Hecho un publish");
                }
                Err(e) => println!("Sistema-Camara: Error al hacer el publish {:?}", e),
            };
        }
    }
}

fn subscribe_to_topics(
    mqtt_client: Arc<Mutex<MQTTClient>>,
    topics: Vec<String>,
) -> Result<(), Error> {
    if let Ok(mut mqtt_client_lock) = mqtt_client.lock() {
        mqtt_client_lock.mqtt_subscribe(topics)?;
    }
    Ok(())
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

// Recibe mensajes de los topics a los que se ha suscrito
pub fn receive_messages_from_subscribed_topics(
    mqtt_client: &Arc<Mutex<MQTTClient>>,
    cameras_cl: &mut ShCamerasType,
) {
    loop {
        if let Ok(mqtt_client) = mqtt_client.lock() {
            match mqtt_client.mqtt_receive_msg_from_subs_topic() {
                //Publish message: Incident
                Ok(msg) => {
                    let incident = Incident::from_bytes(msg.get_payload());
                    println!("ME LLEGO EL INCIDENTE A SISTEMA CAMARAS");
                    manage_incidents(incident, cameras_cl);
                }
                Err(e) => {
                    if !handle_message_receiving_error(e) {
                        break;
                    }
                }
            }
        }
    }
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

fn spawn_abm_cameras_thread(
    shareable_cameras: Arc<Mutex<HashMap<u8, Camera>>>,
    cameras_tx: mpsc::Sender<Vec<u8>>,
    exit_tx: mpsc::Sender<bool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        abm_cameras(&mut shareable_cameras.clone(), cameras_tx, exit_tx);
    })
}

fn spawn_publish_to_topic_thread(
    mqtt_client_sh: Arc<Mutex<MQTTClient>>,
    cameras_rx: mpsc::Receiver<Vec<u8>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        publish_to_topic(mqtt_client_sh, "Cam", cameras_rx);
    })
}

fn spawn_exit_when_asked_thread(
    mqtt_client_sh_clone_1: Arc<Mutex<MQTTClient>>,
    exit_rx: mpsc::Receiver<bool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        exit_when_asked(mqtt_client_sh_clone_1, exit_rx);
    })
}

fn spawn_subscribe_to_topics_thread(
    mqtt_client_sh_clone_2: Arc<Mutex<MQTTClient>>,
    mqtt_client_sh_clone_3: Arc<Mutex<MQTTClient>>,
    cameras_cloned: &mut Arc<Mutex<HashMap<u8, Camera>>>,
) -> JoinHandle<()> {
    let mut cameras_cloned_2 = cameras_cloned.clone();
    thread::spawn(move || {
        let res = subscribe_to_topics(mqtt_client_sh_clone_2, vec!["Inc".to_string()]);
        match res {
            Ok(_) => {
                println!("Sistema-Camara: Subscripción a exitosa");
                receive_messages_from_subscribed_topics(
                    &mqtt_client_sh_clone_3,
                    &mut cameras_cloned_2,
                );
            }
            Err(e) => println!("Sistema-Camara: Error al subscribirse {:?}", e),
        };
        println!("Saliendo del hilo de subscribirme");
    })
}

fn spawn_threads(
    mqtt_client: MQTTClient,
    shareable_cameras: Arc<Mutex<HashMap<u8, Camera>>>,
    cameras_tx: mpsc::Sender<Vec<u8>>,
    cameras_rx: mpsc::Receiver<Vec<u8>>,
    exit_tx: mpsc::Sender<bool>,
    exit_rx: mpsc::Receiver<bool>,
) -> Vec<JoinHandle<()>> {
    let mut children: Vec<JoinHandle<()>> = vec![];

    let mqtt_client_sh = Arc::new(Mutex::new(mqtt_client));
    let mqtt_client_sh_clone_1: Arc<Mutex<MQTTClient>> = Arc::clone(&mqtt_client_sh);
    let mqtt_client_sh_clone_2: Arc<Mutex<MQTTClient>> = Arc::clone(&mqtt_client_sh_clone_1);
    let mqtt_client_sh_clone_3 = mqtt_client_sh_clone_2.clone();

    let mut cameras_cloned = shareable_cameras.clone();

    children.push(spawn_abm_cameras_thread(
        shareable_cameras,
        cameras_tx,
        exit_tx,
    ));
    children.push(spawn_publish_to_topic_thread(mqtt_client_sh, cameras_rx));
    children.push(spawn_exit_when_asked_thread(
        mqtt_client_sh_clone_1,
        exit_rx,
    ));
    children.push(spawn_subscribe_to_topics_thread(
        mqtt_client_sh_clone_2,
        mqtt_client_sh_clone_3,
        &mut cameras_cloned,
    ));

    children
}

fn main() {
    println!("SISTEMA DE CAMARAS\n");

    let broker_addr = get_broker_address();

    match establish_mqtt_broker_connection(&broker_addr) {
        Ok(mqtt_client) => {
            let shareable_cameras = create_cameras();
            sleep(Duration::from_secs(2)); // Sleep to start everything 'at the same time'
            let (cameras_tx, cameras_rx, exit_tx, exit_rx) = create_channels();
            let children = spawn_threads(
                mqtt_client,
                shareable_cameras,
                cameras_tx,
                cameras_rx,
                exit_tx,
                exit_rx,
            );
            join_all_threads(children);
        }
        Err(e) => println!("Error al conectar al broker MQTT: {:?}", e),
    }
}

fn manage_incidents(incident: Incident, cameras_cl: &mut ShCamerasType) {
    // Proceso los incidentes
    let mut incs_being_managed: HashMap<u8, Vec<u8>> = HashMap::new(); // esto puede ser un atributo..., o no.

    if !incs_being_managed.contains_key(&incident.id) {
        procesar_incidente_por_primera_vez(cameras_cl, incident, &mut incs_being_managed);
    } else {
        procesar_incidente_conocido(cameras_cl, incident, &mut incs_being_managed);
    }
}

// Aux: (condición "hasta que" del enunciado).
/// Procesa un incidente cuando un incidente con ese mismo id ya fue recibido anteriormente.
/// Si su estado es resuelto, vuelve el estado de la/s cámara/s que lo atendían, a ahorro de energía.
fn procesar_incidente_conocido(
    cameras_cl: &mut ShCamerasType,
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
                match cameras_cl.lock() {
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
/// en caso afirmativo, cambia estado de la cámara a activo.
fn procesar_incidente_por_primera_vez(
    cameras_cl: &mut ShCamerasType,
    inc: Incident,
    incs_being_managed: &mut HashMap<u8, Vec<u8>>,
) {
    println!("Proceso el incidente {} por primera vez", inc.id);
    // Recorremos cada una de las cámaras, para ver si el inc está en su rango
    match cameras_cl.lock() {
        Ok(mut cams) => {
            for (cam_id, camera) in cams.iter_mut() {
                //let mut _bordering_cams: Vec<Camera> = vec![]; // lindantes

                if camera.will_register(inc.pos()) {
                    println!(
                        "Está en rango de cam: {}, cambiando su estado a activo.",
                        cam_id
                    ); // [] ver lindantes
                    camera.append_to_incs_being_managed(inc.id);
                    incs_being_managed.insert(inc.id, vec![*cam_id]); // podría estar fuera, pero ver orden en q qdan appendeados al vec si hay más de una
                    println!(
                        "  la cámara queda:\n   cam id y lista de incs: {:?}",
                        camera.get_id_e_incs_for_debug_display()
                    );

                    // aux: acá puedo quedarme con los ids de las lindantes
                    // y afuera del if let procesar esto mismo pero para lindantes
                    // Complejidad de eso #revisable (capaz podrían marcarse...) [] ver
                }
            }
        }
        Err(e) => println!("Error lockeando cameras al atender incidentes {:?}", e),
    };
}

fn send_cameras_from_file_to_sist_monitoreo(
    cameras: &mut ShCamerasType,
    camera_tx: &Sender<Vec<u8>>,
) {
    match cameras.lock() {
        Ok(cams) => {
            for camera in (*cams).values() {
                if camera_tx.send(camera.to_bytes()).is_err() {
                    println!("Error al enviar cámara por tx desde hilo abm.");
                }
            }
        }
        Err(_) => println!("Error al tomar lock de cámaras."),
    }
}

fn get_input_abm(prompt: Option<&str>) -> String {
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

fn create_and_send_camera_abm(
    cameras: &mut ShCamerasType,
    camera_tx: &Sender<Vec<u8>>,
    id: u8,
    latitude: f64,
    longitude: f64,
    range: u8,
    border_camera: u8,
) {
    // Crea la nueva cámara con los datos ingresados en el abm
    let mut new_camera = Camera::new(id, latitude, longitude, range, vec![]);
    
    match cameras.lock() {
        Ok(mut cams) => {
            // Recorre las cámaras ya existentes, agregando la nueva cámara como lindante de la que corresponda y viceversa, terminando la creación
            for camera in cams.values_mut() {
                camera.mutually_add_if_bordering(&mut new_camera);        
            }

            // Envía la nueva cámara por tx, para ser publicada por el otro hilo
            if camera_tx.send(new_camera.to_bytes()).is_err() {
                println!("Error al enviar cámara por tx desde hilo abm.");
            }
            // Guarda la nueva cámara
            cams.insert(id, new_camera);
            println!("Cámara agregada con éxito.\n");
        }
        Err(e) => println!("Error tomando lock en agregar cámara abm, {:?}.\n", e),
    };
}

fn add_camera_abm(cameras: &mut ShCamerasType, camera_tx: &Sender<Vec<u8>>) {
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
    let border_camera: u8 = get_input_abm(Some("Ingrese el id de cámara lindante: "))
        .parse()
        .expect("Id no válido");

    create_and_send_camera_abm(
        cameras,
        camera_tx,
        id,
        latitude,
        longitude,
        range,
        border_camera,
    );
}

fn show_cameras_abm(cameras: &mut ShCamerasType) {
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

fn delete_camera_abm(cameras: &mut ShCamerasType, camera_tx: &Sender<Vec<u8>>) {
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

fn exit_program_abm(exit_tx: &Sender<bool>) {
    match exit_tx.send(true) {
        Ok(_) => println!("Saliendo del programa."),
        Err(e) => println!("Error al intentar salir: {:?}", e),
    }
}

fn print_menu_abm() {
    println!(
        "      MENÚ
    1. Agregar cámara
    2. Mostrar cámaras
    3. Eliminar cámara
    4. Salir
    Ingrese una opción:"
    );
}

fn abm_cameras(cameras: &mut ShCamerasType, camera_tx: Sender<Vec<u8>>, exit_tx: Sender<bool>) {
    send_cameras_from_file_to_sist_monitoreo(cameras, &camera_tx);

    loop {
        print_menu_abm();
        let input = get_input_abm(None);

        match &*input {
            "1" => add_camera_abm(cameras, &camera_tx),
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
