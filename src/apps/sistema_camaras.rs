use rustx::apps::camera::Camera;
use rustx::mqtt_client::MQTTClient;
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;
use std::sync::{mpsc, Arc};
use std::thread::{sleep, JoinHandle};
use std::time::Duration;
use std::{fs, thread};
type ShareableCamType = Camera;
type ShCamerasType = Arc<Mutex<HashMap<u8, ShareableCamType>>>;
use rustx::apps::incident::Incident;
//use rustx::apps::properties::Properties;

//use std::env::args;
use std::error::Error;
use std::net::SocketAddr;

fn read_cameras_from_file(filename: &str) -> HashMap<u8, Camera> {
    let mut cameras = HashMap::new();
    let contents = fs::read_to_string(filename).expect("Error al leer el archivo de properties");

    for line in contents.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() == 5 {
            let id: u8 = parts[0].trim().parse().expect("Id no válido");
            let latitude = parts[1].trim().parse().expect("Latitud no válida");
            let longitude = parts[2].trim().parse().expect("Longitud no válida");
            let range = parts[3].trim().parse().expect("Rango no válido");
            let border_cam: u8 = parts[4].trim().parse().expect("Id no válido");
            let vec = vec![border_cam];

            let camera = Camera::new(id, latitude, longitude, range, vec);
            cameras.insert(id, camera);
        }
    }

    cameras
}

///Recibe por consola la dirección IP del cliente y el puerto en el que se desea correr el servidor.
fn load_ip_and_port() -> Result<(String, u16), Box<dyn Error>> {
    let argv = std::env::args().collect::<Vec<String>>();
    if argv.len() != 3 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Cantidad de argumentos inválido. Debe ingresar:  la dirección IP de sistema_camaras y 
            el puerto en el que desea correr el servidor.",
        )));
    }

    let ip_cam = &argv[1];

    let port = match argv[2].parse::<u16>() {
        Ok(port) => port,
        Err(_) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "El puerto proporcionado no es válido",
            )))
        }
    };

    Ok((ip_cam.to_string(), port))
}

pub fn establish_mqtt_broker_connection(
    broker_addr: &SocketAddr,
) -> Result<MQTTClient, Box<dyn std::error::Error>> {
    let client_id = "Sistema-Camaras";
    let mqtt_client_res = MQTTClient::mqtt_connect_to_broker(client_id, broker_addr);
    match mqtt_client_res {
        Ok(mqtt_client) => {
            println!("Cliente: Conectado al broker MQTT.");
            Ok(mqtt_client)
        }
        Err(e) => {
            println!("Sistema-Camara: Error al conectar al broker MQTT: {:?}", e);
            Err(e.into())
        }
    }
}

fn publish_to_topic(mqtt_client: &mut MQTTClient, topic: &str, rx: Receiver<Vec<u8>>) {
    while let Ok(cam_bytes) = rx.recv() {
        let res = mqtt_client.mqtt_publish(topic, &cam_bytes);
        match res {
            Ok(_) => {
                println!("Sistema-Camara: Hecho un publish");
            }
            Err(e) => println!("Sistema-Camara: Error al hacer el publish {:?}", e),
        };
    }
}

fn get_broker_address() -> SocketAddr {
    let (ip, port) = load_ip_and_port().unwrap_or_else(|e| {
        println!("Error al cargar el puerto: {:?}", e);
        std::process::exit(1);
    });

    let broker_addr: String = format!("{}:{}", ip, port);
    broker_addr.parse().expect("Dirección no válida")
}

fn join_all_threads(children: Vec<JoinHandle<()>>) {
    for hijo in children {
        if let Err(e) = hijo.join() {
            eprintln!("Error al esperar el hilo: {:?}", e);
        }
    }
}

fn main() {
    println!("SISTEMA DE CAMARAS\n");

    let broker_addr = get_broker_address();
    let mut children: Vec<JoinHandle<()>> = vec![];

    match establish_mqtt_broker_connection(&broker_addr) {
        Ok(mut mqtt_client) => {
            let cameras: HashMap<u8, Camera> = read_cameras_from_file("./cameras.properties");
            let mut shareable_cameras = Arc::new(Mutex::new(cameras)); // Lo que se comparte es el Cameras completo, x eso lo tenemos que wrappear en arc mutex

            let mut cameras_cloned = shareable_cameras.clone(); // ahora sí es cierto que este clone es el del arc y da una ref (antes sí lo estábamos clonando sin querer)

            // [] aux, probando: un sleep para que empiece todo 'a la vez', y me dé tiempo a levantar las shells
            // y le dé tiempo a conectarse por mqtt, así se van intercalando los hilos a ver si funcionan bien los locks.
            sleep(Duration::from_secs(2));

            let (tx, rx) = mpsc::channel::<Vec<u8>>();

            let handle = thread::spawn(move || {
                abm_cameras(&mut shareable_cameras, tx);
            });
            children.push(handle);

            let handle_2 = thread::spawn(move || {
                publish_to_topic(&mut mqtt_client, "Cam", rx);
            });
            children.push(handle_2);
            // Menú cámaras

            // Atender incidentes
            manage_incidents(&mut cameras_cloned);
        }
        Err(e) => println!("Error al conectar al broker MQTT: {:?}", e),
    }

    join_all_threads(children);
}

fn manage_incidents(cameras_cl: &mut ShCamerasType) {
    // probando, unos incidentes hardcodeados
    let mut read_incs: Vec<Incident> = vec![]; // (a mí no me digan, yo quería programar en castellano, xd)
    let inc1 = Incident::new(1, 1.0, 1.0);
    let inc2 = Incident::new(2, 5.0, 5.0);
    let inc3 = Incident::new(3, 15.0, 15.0);
    let mut inc1_resuelto = Incident::new(1, 1.0, 1.0);
    inc1_resuelto.set_resolved();
    read_incs.push(inc1);
    read_incs.push(inc2);
    read_incs.push(inc3);
    read_incs.push(inc1_resuelto);
    // fin probando

    // Proceso los incidentes
    let mut incs_being_managed: HashMap<u8, Vec<u8>> = HashMap::new(); // esto puede ser un atributo..., o no.

    for inc in read_incs {
        if !incs_being_managed.contains_key(&inc.id) {
            procesar_incidente_por_primera_vez(cameras_cl, inc, &mut incs_being_managed);
        } else {
            procesar_incidente_conocido(cameras_cl, inc, &mut incs_being_managed);
        }
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

fn abm_cameras(cameras: &mut ShCamerasType, camera_tx: Sender<Vec<u8>>) {
    // Envía todas las cámaras al inicio
    match cameras.lock() {
        Ok(cams) => {
            for camera in (*cams).values() {
                // Envío la cámara eliminada por el tx
                if camera_tx.send(camera.to_bytes()).is_err() {
                    println!("Error al enviar cámara por tx desde hilo abm.");
                }
            }
        }
        Err(_) => println!("Error al tomar lock de cámaras."),
    }

    loop {
        println!(
            "      MENÚ
        1. Agregar cámara
        2. Mostrar cámaras
        3. Eliminar cámara
        4. Salir
        Ingrese una opción:"
        );
        let _ = io::stdout().flush();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Error al leer la entrada");

        match input.trim() {
            "1" => {
                print!("Ingrese el ID de la cámara: ");
                let _ = io::stdout().flush();
                let mut read_id = String::new();
                io::stdin()
                    .read_line(&mut read_id)
                    .expect("Error al leer la entrada");
                let id: u8 = read_id.trim().parse().expect("Coordenada X no válida");

                print!("Ingrese Latitud: ");
                let _ = io::stdout().flush();
                let mut read_latitude = String::new();
                io::stdin()
                    .read_line(&mut read_latitude)
                    .expect("Error al leer la entrada");
                let latitude: f64 = read_latitude.trim().parse().expect("Latitud no válida");

                print!("Ingrese Longitud: ");
                let _ = io::stdout().flush();
                let mut read_longitude = String::new();
                io::stdin()
                    .read_line(&mut read_longitude)
                    .expect("Error al leer la entrada");
                let longitude: f64 = read_longitude
                    .trim()
                    .parse()
                    .expect("Coordenada Y no válida");

                print!("Ingrese el rango: ");
                let _ = io::stdout().flush();
                let mut read_range = String::new();
                io::stdin()
                    .read_line(&mut read_range)
                    .expect("Error al leer la entrada");
                let range: u8 = read_range.trim().parse().expect("Rango no válido");

                print!("Ingrese el id de cámara lindante: ");
                let _ = io::stdout().flush();
                let mut read_border_cam = String::new();
                io::stdin()
                    .read_line(&mut read_border_cam)
                    .expect("Error al leer la entrada");
                let border_camera: u8 = read_border_cam.trim().parse().expect("Id no válido");

                // Crea la cámara y la envia
                let new_camera = Camera::new(id, latitude, longitude, range, vec![border_camera]);
                match cameras.lock() {
                    Ok(mut cams) => {
                        if camera_tx.send(new_camera.to_bytes()).is_err() {
                            println!("Error al enviar cámara por tx desde hilo abm.");
                        }
                        cams.insert(id, new_camera);
                        println!("Cámara agregada con éxito.\n");
                    }
                    Err(e) => println!("Error tomando lock en agregar cámara abm, {:?}.\n", e),
                };
            }
            "2" => {
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
            "3" => {
                // Leemos el id de la cámara a eliminar
                print!("Ingrese el ID de la cámara a eliminar: ");
                let _ = io::stdout().flush();
                let mut read_id = String::new();
                io::stdin()
                    .read_line(&mut read_id)
                    .expect("Error al leer la entrada");
                let id: u8 = read_id.trim().parse().expect("Id no válido");

                // Eliminamos la cámara
                match cameras.lock() {
                    Ok(mut cams) => {
                        // Si no estaba en el hashmap, no hago nada, tampoco es error;
                        // else, la elimino, la marco deleted para simplificar la comunicación y la envío
                        if let Some(mut camera_to_delete) = cams.remove(&id) {
                            if camera_to_delete.is_not_deleted() {
                                // (debería dar siempre true)
                                camera_to_delete.delete_camera();
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
            "4" => {
                println!("Saliendo del programa.");
                break;
            }
            _ => {
                println!("Opción no válida. Intente nuevamente.\n");
            }
        }
    }
}
