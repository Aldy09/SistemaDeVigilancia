use rustx::camera::{Camera, self};
use rustx::connect_message::ConnectMessage;
use rustx::mqtt_client::MQTTClient;
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::Mutex;
use std::{fs, thread};

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

            //println!("[DEBUG]")
            let camera = Camera::new(id, coord_x, coord_y, range, vec);
            let shareable_camera = Arc::new(Mutex::new(camera));
            cameras.insert(id, shareable_camera);
        }
    }

    cameras
}

fn connect_and_publish(cameras: &mut Arc<Mutex<HashMap<u8, Arc<Mutex<Camera>>>>>) {
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
            let mut cameras_para_hilo = Arc::clone(cameras); 

            let h_pub = thread::spawn(move || {

                let cams = cameras_para_hilo.lock().unwrap(); // <--- esto lockea 'por siempre', xq viene un loop :(, capaz cambiarlo por un rwlock al mutex de afuera? p q el otro hilo pueda leer
                loop {
                    for (_, camera) in cams.iter() {
                    //for (_, camera) in cams {
                    //for (_, camera) in cameras_para_hilo.as_ref() {
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
                            Err(_) => todo!(),
                        };
                        
                    }
                }

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

    let cameras: HashMap<u8, Arc<Mutex<Camera>>> = read_cameras_from_file("cameras.properties");
    let mut shareable_cameras = Arc::new(Mutex::new(cameras)); // Lo que se comparte es el Cameras completo, x eso lo tenemos que wrappear en arc mutex
    let mut cameras_cloned = shareable_cameras.clone(); // ahora sí es cierto que este clone es el del arc y da una ref (antes sí lo estábamos clonando sin querer)
    // Menú cámaras
    let handle = thread::spawn(move || {
        abm_cameras(&mut shareable_cameras);
    });

    // Publicar cámaras
    let handle_2 = thread::spawn(move || {
        connect_and_publish(&mut cameras_cloned);
    });

    // Manejar incidentes

    // Esperar hijo
    if handle.join().is_err() {
        println!("Error al esperar al hijo.");
    }
    if handle_2.join().is_err() {
        println!("Error al esperar al hijo.");
    }
}

fn abm_cameras(cameras: &mut Arc<Mutex<HashMap<u8, Arc<Mutex<Camera>>>>>) {
    loop {
        println!("1. Agregar cámara");
        println!("2. Mostrar cámaras");
        println!("3. Eliminar cámara");
        println!("4. Salir");
        print!("Ingrese una opción: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Error al leer la entrada");

        match input.trim() {
            "1" => {
                print!("Ingrese el ID de la cámara: ");
                io::stdout().flush().unwrap();
                let mut read_id = String::new();
                io::stdin()
                    .read_line(&mut read_id)
                    .expect("Error al leer la entrada");
                let id: u8 = read_id.trim().parse().expect("Coordenada X no válida");

                print!("Ingrese la coordenada X: ");
                io::stdout().flush().unwrap();
                let mut read_coord_x = String::new();
                io::stdin()
                    .read_line(&mut read_coord_x)
                    .expect("Error al leer la entrada");
                let coord_x: i32 = read_coord_x.trim().parse().expect("Coordenada X no válida");

                print!("Ingrese la coordenada Y: ");
                io::stdout().flush().unwrap();
                let mut read_coord_y = String::new();
                io::stdin()
                    .read_line(&mut read_coord_y)
                    .expect("Error al leer la entrada");
                let coord_y: i32 = read_coord_y.trim().parse().expect("Coordenada Y no válida");

                print!("Ingrese el rango: ");
                io::stdout().flush().unwrap();
                let mut read_range = String::new();
                io::stdin()
                    .read_line(&mut read_range)
                    .expect("Error al leer la entrada");
                let range: u8 = read_range.trim().parse().expect("Rango no válido");

                print!("Ingrese el id de cámara lindante: ");
                io::stdout().flush().unwrap();
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
                                if !(camera.lock().unwrap().deleted) {
                                    let camera = camera.lock().unwrap();
                                    camera.display();
                                }
                            }
                        }
                    },
                    Err(_) => todo!(),
                }
            }
            "3" => {
                // Leemos el id de la cámara a eliminar
                print!("Ingrese el ID de la cámara a eliminar: ");
                io::stdout().flush().unwrap();
                let mut read_id = String::new();
                io::stdin()
                    .read_line(&mut read_id)
                    .expect("Error al leer la entrada");
                let id: u8 = read_id.trim().parse().expect("Id no válido");

                // Eliminamos la cámara
                match cameras.lock(){
                    Ok(mut cams) => {
                        match cams.remove(&id) { // [] obs, ToDo: habíamos dicho que era un borrado lógico
                            Some(_) => println!("Cámara eliminada con éxito.\n"),
                            None => println!("No se encontró la cámara con el ID especificado.\n"),
                        }
                    },
                    Err(e) => println!("Error tomando lock en baja abm, {:?}.\n", e),
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
