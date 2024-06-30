use std::{
    collections::HashMap,
    net::TcpStream,
    sync::{mpsc, Mutex},
};

use rustx::{
    apps::{
        common_clients::get_broker_address,
        sist_camaras::{camera::Camera, sistema_camaras::SistemaCamaras},
    },
    logging::structs_to_save_in_logger::StructsToSaveInLogger,
    mqtt::{
        client::{
            mqtt_client::{self, MQTTClient},
            mqtt_client_listener,
            mqtt_client_server_connection::MqttClientConnection,
        },
        messages::publish_message::{self, PublishMessage},
    },
};

type Channels = (
    mpsc::Sender<Vec<u8>>,
    mpsc::Receiver<Vec<u8>>,
    mpsc::Sender<bool>,
    mpsc::Receiver<bool>,
    mpsc::Sender<StructsToSaveInLogger>,
    mpsc::Receiver<StructsToSaveInLogger>,
    mpsc::Sender<PublishMessage>,
    mpsc::Receiver<PublishMessage>,
);

fn create_channels() -> Channels {
    let (cameras_tx, cameras_rx) = mpsc::channel::<Vec<u8>>();
    let (exit_tx, exit_rx) = mpsc::channel::<bool>();
    let (logger_tx, logger_rx) = mpsc::channel::<StructsToSaveInLogger>();
    let (publish_message_tx, publish_message_rx) = mpsc::channel::<PublishMessage>();
    (
        cameras_tx,
        cameras_rx,
        exit_tx,
        exit_rx,
        logger_tx,
        logger_rx,
        publish_message_tx,
        publish_message_rx,
    )
}

fn create_cameras() -> Arc<Mutex<HashMap<u8, Camera>>> {
    let cameras: HashMap<u8, Camera> = read_cameras_from_file("./cameras.properties");
    Arc::new(Mutex::new(cameras))
}

fn establish_mqtt_broker_connection(broker_addr: &SocketAddr) -> Result<(TcpStream), Error> {
    let client_id = "Sistema-Camaras";
    let handshake_result = MqttClientConnection::mqtt_connect_to_broker(client_id, broker_addr);
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
}

fn main() {
    let broker_addr = get_broker_address();
    let (
        cameras_tx,
        cameras_rx,
        exit_tx,
        exit_rx,
        logger_tx,
        logger_rx,
        publish_message_tx,
        publish_message_rx,
    ) = create_channels();
    let cameras = create_cameras();

    match establish_mqtt_broker_connection(&broker_addr) {
        Ok(stream) => {
            let mut handlers = Vec::<JoinHandle<()>>::new();
            let mqtt_client_listener =
                MQTTClientListener::new(stream.try_clone().unwrap(), publish_message_tx);
            let mqtt_client = MQTTClient::new(stream, mqtt_client_listener);
            let sistema_camaras = SistemaCamaras::new(
                cameras_tx,
                logger_tx,
                exit_tx,
                cameras,
                publish_message_rx,
                mqtt_client,
            );

            handlers.push(thread::spawn(move || {
                let _ = mqtt_client_listener.read_from_server();
            }));
            handlers.push(sistema_camaras.spawn_threads(cameras_rx, logger_rx, exit_rx));

            join_all_threads(handlers);
        }
        Err(e) => println!("Error al conectar al broker MQTT: {:?}", e),
    }
}
