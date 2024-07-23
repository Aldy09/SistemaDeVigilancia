use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
    thread::{self},
};

use std::io::Error;

use rustx::{
    apps::{
        common_clients::{get_broker_address, join_all_threads},
        sist_camaras::{
            camera::Camera, manage_stored_cameras::read_cameras_from_file,
            sistema_camaras::SistemaCamaras,
        },
    },
    logging::string_logger::StringLogger,
    mqtt::{
        client::{
            mqtt_client::MQTTClient, mqtt_client_listener::MQTTClientListener,
            mqtt_client_server_connection::mqtt_connect_to_broker,
        },
        messages::publish_message::PublishMessage,
    },
};

type Channels = (
    mpsc::Sender<Vec<u8>>,
    mpsc::Receiver<Vec<u8>>,
    mpsc::Sender<bool>,
    mpsc::Receiver<bool>,
    mpsc::Sender<PublishMessage>,
    mpsc::Receiver<PublishMessage>,
);

fn create_channels() -> Channels {
    let (cameras_tx, cameras_rx) = mpsc::channel::<Vec<u8>>();
    let (exit_tx, exit_rx) = mpsc::channel::<bool>();
    let (publish_message_tx, publish_message_rx) = mpsc::channel::<PublishMessage>();
    (
        cameras_tx,
        cameras_rx,
        exit_tx,
        exit_rx,
        publish_message_tx,
        publish_message_rx,
    )
}

fn create_cameras() -> Arc<Mutex<HashMap<u8, Camera>>> {
    let cameras: HashMap<u8, Camera> = read_cameras_from_file("./cameras.properties");
    Arc::new(Mutex::new(cameras))
}

/*
// Aux: Esta función no es necesaria, tiene un match y solo devuelve Ok y Err, y afuera se vuelve a hacer el match.
// Se puede hacer el match afuera directamente entonces.
fn establish_mqtt_broker_connection(broker_addr: &SocketAddr) -> Result<TcpStream, Error> {
    let client_id = "Sistema-Camaras";
    let handshake_result = mqtt_connect_to_broker(client_id, broker_addr);
    match handshake_result {
        Ok(stream) => {
            println!("Cliente: Conectado al broker MQTT.");
            Ok(stream)
        }
        Err(e) => {
            println!("Sistema-Camara: Error al conectar al broker MQTT: {:?}", e);
            Err(e)
        }
    }
}*/

fn get_formatted_app_id() -> String {
    String::from("Sistema-Camaras")
}

fn main() -> Result<(), Error>{
    let broker_addr = get_broker_address();
    let (
        cameras_tx,
        cameras_rx,
        exit_tx,
        exit_rx,
        publish_message_tx,
        publish_message_rx,
    ) = create_channels();
    let cameras = create_cameras();

    // Se crean y configuran ambos extremos del string logger
    let (logger, handle_logger) = StringLogger::create_logger(get_formatted_app_id());

    let client_id = get_formatted_app_id();
    match mqtt_connect_to_broker(client_id.as_str(), &broker_addr) {
        Ok(stream) => {
            let mut mqtt_client_listener =
                MQTTClientListener::new(stream.try_clone()?, publish_message_tx);
            let mqtt_client: MQTTClient = MQTTClient::new(stream, mqtt_client_listener.clone());
            println!("Cliente: Conectado al broker MQTT.");

            let mut sistema_camaras = SistemaCamaras::new(cameras_tx, exit_tx, cameras, logger);

            let handler_1 = thread::spawn(move || {
                let _ = mqtt_client_listener.read_from_server();
            });

            let mut handlers = sistema_camaras.spawn_threads(
                cameras_rx,
                exit_rx,
                publish_message_rx,
                mqtt_client,
            );

            handlers.push(handler_1);
            join_all_threads(handlers);
        }
        Err(e) => println!("Error al conectar al broker MQTT: {:?}", e),
    }

    // Se espera al hijo para el logger writer
    if handle_logger.join().is_err() {
        println!("Error al esperar al hijo para string logger writer.")
    }

    Ok(())
}
