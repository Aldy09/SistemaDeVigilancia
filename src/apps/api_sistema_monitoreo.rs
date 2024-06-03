use std::{
    error::Error,
    net::SocketAddr,
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

use std::sync::mpsc::Receiver;

use crossbeam::channel::{self, Sender};

use crate::{messages::publish_message::PublishMessage, mqtt_client::MQTTClient};

use super::{camera::Camera, incident::Incident, ui_sistema_monitoreo::UISistemaMonitoreo};

#[derive(Debug)]
pub struct SistemaMonitoreo {
    pub incidents: Arc<Mutex<Vec<Incident>>>,
    pub camera_tx: Sender<Camera>,
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

/// Lee el IP del cliente y el puerto en el que el cliente se va a conectar al servidor.
pub fn load_ip_and_port() -> Result<(String, u16), Box<dyn Error>> {
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

pub fn establish_mqtt_broker_connection(
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

pub fn finalize_mqtt_client(mqtt_client: &Arc<Mutex<MQTTClient>>) {
    if let Ok(mut mqtt_client) = mqtt_client.lock() {
        mqtt_client.finalizar();
    }
}

pub fn publish_incident(incident: Incident, mqtt_client: &Arc<Mutex<MQTTClient>>) {
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

impl SistemaMonitoreo {
    pub fn new() -> Self {
        let (tx_camera, rx_camera) = channel::unbounded();
        let (tx, rx) = mpsc::channel::<Incident>();
        let mut children: Vec<JoinHandle<()>> = vec![];
        let broker_addr = get_broker_address();

        let sistema_monitoreo = Self {
            incidents: Arc::new(Mutex::new(Vec::new())),
            camera_tx: tx_camera,
        };

        match establish_mqtt_broker_connection(&broker_addr) {
            Ok(mqtt_client) => {
                let mqtt_client_sh = Arc::new(Mutex::new(mqtt_client));
                let mqtt_client_sh_clone: Arc<Mutex<MQTTClient>> = Arc::clone(&mqtt_client_sh);

                let send_subscribe_thread =
                    sistema_monitoreo.spawn_subscribe_to_topics_thread(mqtt_client_sh);
                children.push(send_subscribe_thread);

                let mqtt_client_incident_sh_clone = Arc::clone(&mqtt_client_sh_clone);

                let send_incidents_thread = sistema_monitoreo
                    .spawn_send_incidents_thread(mqtt_client_incident_sh_clone, rx);
                children.push(send_incidents_thread);
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
            Box::new(|cc| {
                Box::new(UISistemaMonitoreo::new(
                    cc.egui_ctx.clone(),
                    tx_clone,
                    rx_camera,
                ))
            }),
        );

        join_all_threads(children);

        sistema_monitoreo
    }

    pub fn spawn_send_incidents_thread(
        &self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        rx: Receiver<Incident>,
    ) -> JoinHandle<()> {
        thread::spawn(move || loop {
            while let Ok(msg) = rx.recv() {
                publish_incident(msg, &mqtt_client);
            }
        })
    }

    pub fn clone_ref(&self) -> Self {
        Self {
            incidents: self.incidents.clone(),
            camera_tx: self.camera_tx.clone(),
        }
    }

    pub fn spawn_subscribe_to_topics_thread(
        &self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
    ) -> JoinHandle<()> {
        let self_clone = self.clone_ref();
        thread::spawn(move || {
            self_clone.subscribe_to_topics(mqtt_client);
        })
    }

    pub fn finalize_mqtt_client(&self, mqtt_client: &Arc<Mutex<MQTTClient>>) {
        if let Ok(mut mqtt_client) = mqtt_client.lock() {
            mqtt_client.finalizar();
        }
    }

    pub fn publish_incident(&self, incident: Incident, mqtt_client: &Arc<Mutex<MQTTClient>>) {
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

    pub fn subscribe_to_topics(&self, mqtt_client: Arc<Mutex<MQTTClient>>) {
        self.subscribe_to_cam_topic(&mqtt_client);
        self.receive_messages_from_subscribed_topics(&mqtt_client);
        finalize_mqtt_client(&mqtt_client);
    }

    pub fn subscribe_to_cam_topic(&self, mqtt_client: &Arc<Mutex<MQTTClient>>) {
        if let Ok(mut mqtt_client) = mqtt_client.lock() {
            let res_sub = mqtt_client.mqtt_subscribe(vec![(String::from("Cam"))]);
            match res_sub {
                Ok(_) => println!("Cliente: Hecho un subscribe"),
                Err(e) => println!("Cliente: Error al hacer un subscribe: {:?}", e),
            }
        }
    }

    pub fn receive_messages_from_subscribed_topics(&self, mqtt_client: &Arc<Mutex<MQTTClient>>) {
        loop {
            if let Ok(mqtt_client) = mqtt_client.lock() {
                match mqtt_client.mqtt_receive_msg_from_subs_topic() {
                    Ok(msg) => self.handle_received_camera(msg),
                    Err(e) => {
                        if !handle_message_receiving_error(e) {
                            break;
                        }
                    }
                }
            }
        }
    }
    pub fn handle_received_camera(&self, msg: PublishMessage) {
        println!("Cliente: Recibo estos msg_bytes: {:?}", msg);
        let camera_recibida = Camera::from_bytes(&msg.get_payload());
        println!("Cliente: Recibo cámara: {:?}", camera_recibida);
        let res_send = self.camera_tx.send(camera_recibida);
        println!("ENVIANDO POR TX LA CAMARA EN API SISTEMA MONITOREO");
        
        if let Err(e) = res_send {
            println!("Error al enviar la cámara: {:?}", e);
        }
    }

    pub fn add_incident(&mut self, incident: Incident) {
        self.incidents.lock().unwrap().push(incident);
    }

    pub fn get_incidents(&mut self) -> Arc<Mutex<Vec<Incident>>> {
        self.incidents.clone()
    }

    pub fn generate_new_incident_id(&self) -> u8 {
        let mut new_inc_id: u8 = 0;
        if let Ok(incidents) = self.incidents.lock() {
            new_inc_id = (incidents.len() + 1) as u8;
        }
        new_inc_id
    }
}

impl Default for SistemaMonitoreo {
    fn default() -> Self {
        Self::new()
    }
}
