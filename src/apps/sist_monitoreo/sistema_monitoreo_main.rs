use std::net::TcpStream;

use rustx::apps::sist_monitoreo::sistema_monitoreo::{self, SistemaMonitoreo};

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
    let (logger_tx, logger_rx) = mpsc::channel::<StructsToSaveInLogger>();
    let (publish_message_tx, publish_message_rx) = mpsc::channel::<PublishMessage>();
    let (egui_tx, egui_rx) = mpsc::channel::<PublishMessage>();
    (
        cameras_tx,
        cameras_rx,
        logger_tx,
        logger_rx,
        publish_message_tx,
        publish_message_rx,
        egui_tx,
        egui_rx,
    )
}

pub fn establish_mqtt_broker_connection(broker_addr: &SocketAddr) -> Result<TcpStream, Error> {
    let client_id = "Sistema-Monitoreo";
    let handshake_result = mqtt_connect_to_broker(client_id, broker_addr);
    match handshake_result {
        Ok(stream) => {
            println!("Cliente: Conectado al broker MQTT.");
            Ok(stream)
        }
        Err(e) => {
            println!(
                "Sistema-Monitoreo: Error al conectar al broker MQTT: {:?}",
                e
            );
            Err(e)
        }
    }
}

fn main() {
    let broker_addr = get_broker_address();

    let (
        cameras_tx,
        cameras_rx,
        logger_tx,
        logger_rx,
        publish_message_tx,
        publish_message_rx, egui_tx, egui_rx,
    ) = create_channels();



    match establish_mqtt_broker_connection(&broker_addr) {
        Ok(stream) => {
            let mut handlers = Vec::<JoinHandle<()>>::new();
            let mqtt_client_listener =
                MQTTClientListener::new(stream.try_clone().unwrap(), publish_message_tx);
            let mqtt_client: MQTTClient = MQTTClient::new(stream, mqtt_client_listener);
            let sistema_monitoreo = SistemaMonitoreo::new(logger_tx, mqtt_client, egui_tx);

            handlers.push(thread::spawn(move || {
                let _ = mqtt_client_listener.read_from_server();
            }));

            handlers.push(sistema_monitoreo.spawn_threads(logger_rx, publish_message_rx, egui_rx));

            join_all_threads(child_threads);
        }
        Err(e) => println!(
            "Error al establecer la conexión con el broker MQTT: {:?}",
            e
        ),
    }
}
