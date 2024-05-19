use rustx::apps::camera::Camera;
use rustx::apps::camera_state::CameraState;
use rustx::connect_message::ConnectMessage;
use rustx::mqtt_client::MQTTClient;
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::Mutex;
use std::{fs, thread};
type ShareableCamType = Arc<Mutex<Camera>>;
type ShCamerasType = Arc<Mutex<HashMap<u8, ShareableCamType>>>;
use rustx::apps::incident::Incident;

fn read_cameras_from_file(filename: &str) -> HashMap<u8, Arc<Mutex<Camera>>> {
    let mut cameras = HashMap::new();
    let contents = fs::read_to_string(filename).expect("Error al leer el archivo de properties");

    for line in contents.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() == 5 {
            let id: u8 = parts[0].trim().parse().expect("Id no válido");
            let coord_x = parts[1].trim().parse().expect("Coordenada X no válida");
            let coord_y = parts[2].trim().parse().expect("Coordenada Y no válida");
            let range = parts[3].trim().parse().expect("Rango no válido");
            let border_cam: u8 = parts[4].trim().parse().expect("Id no válido");
            let vec = vec![border_cam];

            let camera = Camera::new(id, coord_x, coord_y, range, vec);
            let shareable_camera = Arc::new(Mutex::new(camera));
            cameras.insert(id, shareable_camera);
        }
    }

    cameras
}

fn connect_and_publish(cameras: &mut ShCamerasType) {
    let ip = "127.0.0.1".to_string();
    let port = 9090;
    let broker_addr = format!("{}:{}", ip, port)
        .parse()
        .expect("Dirección no válida");
    let mut connect_msg = ConnectMessage::new(
        0x01 << 4, // Me fijé y el fixed header no estaba shifteado, el message type tiene que quedar en los 4 bits más signifs del primer byte (toDo: arreglarlo para el futuro)
        // toDo: obs: además, al propio new podría agregarlo, no? para no tener yo que acordarme qué tipo es cada mensaje.
        "rust-client",
        None, // will_topic
        None, // will_message
        Some("sistema-monitoreo"),
        Some("rustx123"),
    );

    // Cliente usa funciones connect, publish, y subscribe de la lib.
    let mqtt_client_res = MQTTClient::connect_to_broker(&broker_addr, &mut connect_msg);
    match mqtt_client_res {
        Ok(mqtt_client) => {
            //info!("Conectado al broker MQTT."); //
            println!("Sistema-Camara: Conectado al broker MQTT.");

            let mut mqtt_client_para_hijo = mqtt_client.mqtt_clone();
            let cameras_para_hilo = Arc::clone(cameras); 

            let h_pub = thread::spawn(move || {

                if let Ok(cams) = cameras_para_hilo.lock(){ // [] <--- esto lockea 'por siempre', xq viene un loop :(, capaz cambiarlo por un rwlock al mutex de afuera? p q el otro hilo pueda leer
                    loop {
                        for (_, camera) in cams.iter() {
                            match camera.lock() { // como no guardo en una variable lo que me devuelve el lock, el lock se dropea al cerrar esta llave
                                Ok(mut cam) => {
                                    if !(cam.sent) {
                                        let res = mqtt_client_para_hijo.mqtt_publish("Cam", &cam.to_bytes());
                                        match res {
                                            Ok(_) => {
                                                println!("Sistema-Camara: Hecho un publish exitosamente");
                                                cam.sent=true;
                                        },
                                            Err(e) => println!("Sistema-Camara: Error al hacer el publish {:?}", e),
                                    };
                                }},
                                Err(e) => println!("Sistema-Camara: Error al tomar lock de una cámara: {:?}", e),
                            };
                            
                        }
                    };
            };

            });

            // Esperar a los hijos
            if h_pub.join().is_err() {
                println!("Error al esperar a hijo publisher.");
            }
        }
        Err(e) => println!("Sistema-Camara: Error al conectar al broker MQTT: {:?}", e),
    }
}



fn main() {
    println!("SISTEMA DE CAMARAS\n");

    let cameras: HashMap<u8, Arc<Mutex<Camera>>> = read_cameras_from_file("./cameras.properties");
    let mut shareable_cameras = Arc::new(Mutex::new(cameras)); // Lo que se comparte es el Cameras completo, x eso lo tenemos que wrappear en arc mutex
    let mut cameras_cloned = shareable_cameras.clone(); // ahora sí es cierto que este clone es el del arc y da una ref (antes sí lo estábamos clonando sin querer)
    let mut cameras_cloned_2 = shareable_cameras.clone();
    // Menú cámaras
    let handle = thread::spawn(move || {
        abm_cameras(&mut shareable_cameras);
    });

    // Publicar cámaras
    let handle_2 = thread::spawn(move || {
        connect_and_publish(&mut cameras_cloned);
    });

    // Atender incidentes
    manage_incidents(&mut cameras_cloned_2);

    // Esperar hijo
    if handle.join().is_err() {
        println!("Error al esperar al hijo.");
    }
    if handle_2.join().is_err() {
        println!("Error al esperar al hijo.");
    }
}

fn manage_incidents(cameras_cl: &mut ShCamerasType) {
    // probando, unos incidentes hardcodeados
    let mut read_incs : Vec<Incident> = vec![]; // (a mí no me digan, yo quería programar en castellano, xd)
    let inc1 = Incident::new(1, 1, 1);
    let inc2 = Incident::new(2, 5, 5);
    let inc3 = Incident::new(3, 15, 15);
    let mut inc1_resuelto = Incident::new(1, 1, 1);
    inc1_resuelto.set_resolved();
    read_incs.push(inc1);
    read_incs.push(inc2);
    read_incs.push(inc3);
    read_incs.push(inc1_resuelto);
    // fin probando

    // Proceso los incidentes
    let mut incs_being_managed: HashMap<u8, Vec<u8>> = HashMap::new(); // esto puede ser un atributo..., o no.

    for inc in read_incs {
        if !incs_being_managed.contains_key(&inc.id){
            procesar_incidente_por_primera_vez(cameras_cl, inc, &mut incs_being_managed);
        } else {
            procesar_incidente_conocido(cameras_cl, inc, &mut incs_being_managed);
        }
    }  
}

// Aux: (condición "hasta que" del enunciado).
/// Procesa un incidente cuando un incidente con ese mismo id ya fue recibido anteriormente.
/// Si su estado es resuelto, vuelve el estado de la/s cámara/s que lo atendían, a ahorro de energía.
fn procesar_incidente_conocido(cameras_cl: &mut ShCamerasType, inc: Incident, incs_being_managed: &mut HashMap<u8, Vec<u8>>) {

    if inc.is_resolved() {
        println!("Recibo el incidente {} de nuevo, y ahora viene con estado resuelto.", inc.id);
        // Busco la/s cámara/s que atendían este incidente
        if let Some(cams_managing_inc) = incs_being_managed.get(&inc.id){ // sé que existe, por el if de más arriba

            // Cambio el estado de las cámaras que lo manejaban, otra vez a ahorro de energía
            // solamente si el incidente en cuestión era el único que manejaban (si tenía más incidentes en rango, sigue estando activa)
            for camera_id in cams_managing_inc {
                match cameras_cl.lock() {
                    Ok(cams) => {
                        if let Ok(mut cam) = cams[camera_id].lock() {
                            // Actualizo las cámaras en cuestión
                            cam.remove_from_incs_being_managed(inc.id);
                            if cam.empty_incs_list() {
                                cam.set_state_to(CameraState::SavingMode);
                            }
                            println!("  la cámara queda:\n   cam id y lista de incs: {:?}", cam.get_id_e_incs_for_debug_display());
                        };
                    },
                    Err(_) => println!("Error al tomar lock de cámaras para volver estado a ahorro energía."),
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
fn procesar_incidente_por_primera_vez(cameras_cl: &mut ShCamerasType, inc: Incident, incs_being_managed: &mut HashMap<u8, Vec<u8>>) {
    println!("Proceso el incidente {} por primera vez", inc.id);
    // Recorremos cada una de las cámaras, para ver si el inc está en su rango
    match cameras_cl.lock() {
        Ok(cams) => {
            for (cam_id, camera) in cams.iter(){
                //let mut _bordering_cams: Vec<Camera> = vec![]; // lindantes
                if let Ok(mut cam) = camera.lock() {
                    if cam.will_register(inc.pos()) {
                        println!("Está en rango de cam: {}, cambiando su estado a activo.", cam_id); // [] ver lindantes
                        cam.set_state_to(CameraState::Active);
                        incs_being_managed.insert(inc.id, vec![*cam_id]); // podría estar fuera, pero ver orden en q qdan appendeados al vec si hay más de una
                        cam.append_to_incs_being_managed(inc.id);
                        println!("  la cámara queda:\n   cam id y lista de incs: {:?}", cam.get_id_e_incs_for_debug_display());

                        // aux: acá puedo quedarme con los ids de las lindantes
                        // y afuera del if let procesar esto mismo pero para lindantes
                        // Complejidad de eso #revisable (capaz podrían marcarse...) [] ver
                    }
                };
            }
        },
        Err(e) => println!("Error lockeando cameras al atender incidentes {:?}", e),
    };
}

fn abm_cameras(cameras: &mut ShCamerasType) {
    loop {
        println!("1. Agregar cámara");
        println!("2. Mostrar cámaras");
        println!("3. Eliminar cámara");
        println!("4. Salir");
        print!("Ingrese una opción: ");
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

                print!("Ingrese la coordenada X: ");
                let _ = io::stdout().flush();
                let mut read_coord_x = String::new();
                io::stdin()
                    .read_line(&mut read_coord_x)
                    .expect("Error al leer la entrada");
                let coord_x: u8 = read_coord_x.trim().parse().expect("Coordenada X no válida");

                print!("Ingrese la coordenada Y: ");
                let _ = io::stdout().flush();
                let mut read_coord_y = String::new();
                io::stdin()
                    .read_line(&mut read_coord_y)
                    .expect("Error al leer la entrada");
                let coord_y: u8 = read_coord_y.trim().parse().expect("Coordenada Y no válida");

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

                let new_camera = Camera::new(id, coord_x, coord_y, range, vec![border_camera]);
                let shareable_camera = Arc::new(Mutex::new(new_camera));
                match cameras.lock(){
                    Ok(mut cams) => {
                        cams.insert(id, shareable_camera);
                        println!("Cámara agregada con éxito.\n");
                    },
                    Err(e) => println!("Error tomando lock en agregar cámara abm, {:?}.\n", e),
                };
            }
            "2" => {
                // Mostramos todas las cámaras
                println!("Cámaras registradas:\n");
                match cameras.lock() {
                    Ok(cams) => {
                        for camera in (*cams).values() {
                            {
                                match camera.lock() {
                                    Ok(cam) => {
                                        // Si no está marcada borrada, mostrarla
                                        if !cam.deleted {
                                            cam.display();
                                        };
                                    },
                                    Err(_) => println!("Error al tomar lock de la cámara."),
                                }
                            }
                        }
                    },
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

                // Eliminamos la cámara, es un borrado lógico para simplificar la comunicación
                match cameras.lock(){
                    Ok(cams) => {
                        match cams[&id].lock(){
                            Ok(mut cam_a_eliminar) => {
                                
                                // Si ya estaba deleted, no hago nada, tampoco es error; else, la marco deleted
                                if !cam_a_eliminar.deleted {
                                    cam_a_eliminar.deleted = true;
                                };
                                println!("Cámara eliminada con éxito.\n");
                            },
                            Err(_) => println!("Error al tomar lock de cámara a eliminar."),
                        };           
                    },
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
