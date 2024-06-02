use rustx::apps::camera::Camera;
use rustx::apps::incident::Incident;
use rustx::apps::ui_sistema_monitoreo::UISistemaMonitoreo;
use rustx::messages::publish_message::PublishMessage;
use rustx::mqtt_client::MQTTClient;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle}; // Import the `SistemaMonitoreo` type


/// Lee el IP del cliente y el puerto en el que el cliente se va a conectar al servidor.
fn load_ip_and_port() -> Result<(String, u16), Box<dyn Error>> {
    let argv = std::env::args().collect::<Vec<String>>();
    if argv.len() != 3 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Cantidad de argumentos inválido. Debe ingresar: la dirección IP del sistema monitoreo y 
            el puerto en el que desea correr el servidor.",
        )));
    }
    let ip = &argv[1];
    let port = match argv[2].parse::<u16>() {
        Ok(port) => port,
        Err(_) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "El puerto proporcionado no es válido",
            )))
        }
    };

    Ok((ip.to_string(), port))
}

fn establish_mqtt_broker_connection(
    broker_addr: &SocketAddr,
) -> Result<MQTTClient, Box<dyn std::error::Error>> {
    let client_id = "Sistema-Monitoreo";
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

fn subscribe_to_topics(mqtt_client: Arc<Mutex<MQTTClient>>) {
    subscribe_to_cam_topic(&mqtt_client);
    receive_messages_from_subscribed_topics(&mqtt_client);
    finalize_mqtt_client(&mqtt_client);
}

fn subscribe_to_cam_topic(mqtt_client: &Arc<Mutex<MQTTClient>>) {
    if let Ok(mut mqtt_client) = mqtt_client.lock() {
        let res_sub = mqtt_client.mqtt_subscribe(vec![(String::from("Cam"))]);
        match res_sub {
            Ok(_) => println!("Cliente: Hecho un subscribe"),
            Err(e) => println!("Cliente: Error al hacer un subscribe: {:?}", e),
        }
    }
}

fn receive_messages_from_subscribed_topics(mqtt_client: &Arc<Mutex<MQTTClient>>) {
    loop {
        if let Ok(mqtt_client) = mqtt_client.lock() {
            match mqtt_client.mqtt_receive_msg_from_subs_topic() {
                Ok(msg) => handle_received_message(msg),
                Err(e) => {
                    if !handle_message_receiving_error(e) {
                        break;
                    }
                }
            }
        }
    }
}

fn handle_received_message(msg: PublishMessage) {
    println!("Cliente: Recibo estos msg_bytes: {:?}", msg);
    let camera_recibida = Camera::from_bytes(&msg.get_payload());
    println!("Cliente: Recibo cámara: {:?}", camera_recibida);
}

fn handle_message_receiving_error(e: std::io::Error) -> bool {
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

fn finalize_mqtt_client(mqtt_client: &Arc<Mutex<MQTTClient>>) {
    if let Ok(mut mqtt_client) = mqtt_client.lock() {
        mqtt_client.finalizar();
    }
}

fn publish_incident(
    incident: Incident,
    mqtt_client: &Arc<Mutex<MQTTClient>>,
) {
    println!("Sistema-Monitoreo: Publicando incidente.");

    // Hago el publish
    if let Ok(mut mqtt_client) = mqtt_client.lock() {
        let res = mqtt_client.mqtt_publish("Inc", &incident.to_bytes());
        match res {
            Ok(_) => {
                println!("Sistema-Monitoreo: Ha hecho un publish");
            }
            Err(e) => {
                println!("Sistema-Monitoreo: Error al hacer el publish {:?}", e)
            }
        };
    }
}

    

fn main() {
    env_logger::init();
    let broker_addr = get_broker_address();

    let mut hijos: Vec<JoinHandle<()>> = vec![];
    let (tx, rx) = mpsc::channel::<Incident>();


    match establish_mqtt_broker_connection(&broker_addr) {
        Ok(mqtt_client) => {
            let mqtt_client_sh = Arc::new(Mutex::new(mqtt_client));
            let mqtt_client_sh_clone: Arc<Mutex<MQTTClient>> = Arc::clone(&mqtt_client_sh);

            let send_subscribe_thread = spawn_subscribe_to_topics_thread(mqtt_client_sh);
            hijos.push(send_subscribe_thread);

            let mqtt_client_incident_sh_clone = Arc::clone(&mqtt_client_sh_clone);

            let send_incidents_thread =
                spawn_send_incidents_thread(mqtt_client_incident_sh_clone, rx);
            hijos.push(send_incidents_thread);
        }
        Err(e) => println!(
            "Error al establecer la conexión con el broker MQTT: {:?}",
            e
        ),
    }

    let tx_clone = tx.clone();

    let _ = eframe::run_native(
        "Sistema Monitoreo",
        Default::default(),
        Box::new(|cc| Box::new(UISistemaMonitoreo::new(cc.egui_ctx.clone(), tx_clone))),
    );

    join_all_threads(hijos);
}

fn get_broker_address() -> SocketAddr {
    let (ip, port) = load_ip_and_port().unwrap_or_else(|e| {
        println!("Error al cargar el puerto: {:?}", e);
        std::process::exit(1);
    });

    let broker_addr: String = format!("{}:{}", ip, port);
    broker_addr.parse().expect("Dirección no válida")
}

fn spawn_subscribe_to_topics_thread(mqtt_client: Arc<Mutex<MQTTClient>>) -> JoinHandle<()> {
    thread::spawn(move || {
        subscribe_to_topics(mqtt_client);
    })
}

fn spawn_send_incidents_thread(
    mqtt_client: Arc<Mutex<MQTTClient>>,
    rx: Receiver<Incident>
) -> JoinHandle<()> {
    thread::spawn(move || loop {
        while let Ok(msg) = rx.recv() {
            publish_incident(msg, &mqtt_client);
        }
    })
}

fn join_all_threads(hijos: Vec<JoinHandle<()>>) {
    for hijo in hijos {
        if let Err(e) = hijo.join() {
            eprintln!("Error al esperar el hilo: {:?}", e);
        }
    }
}
