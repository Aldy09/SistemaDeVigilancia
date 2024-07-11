use std::io::Error;
use std::{
    net::{SocketAddr, TcpStream},
    sync::mpsc,
    thread::{self},
};

use crossbeam_channel::unbounded;
use rustx::{
    apps::{
        common_clients::{get_broker_address, join_all_threads},
        sist_monitoreo::sistema_monitoreo::SistemaMonitoreo,
    },
    logging::structs_to_save_in_logger::StructsToSaveInLogger,
    mqtt::{
        client::{
            mqtt_client::MQTTClient, mqtt_client_listener::MQTTClientListener,
            mqtt_client_server_connection::mqtt_connect_to_broker,
        },
        messages::publish_message::PublishMessage,
    },
};

type Channels = (
    mpsc::Sender<StructsToSaveInLogger>,
    mpsc::Receiver<StructsToSaveInLogger>,
    mpsc::Sender<PublishMessage>,
    mpsc::Receiver<PublishMessage>,
    crossbeam_channel::Sender<PublishMessage>,
    crossbeam_channel::Receiver<PublishMessage>,
);

fn create_channels() -> Channels {
    let (logger_tx, logger_rx) = mpsc::channel::<StructsToSaveInLogger>();
    let (publish_message_tx, publish_message_rx) = mpsc::channel::<PublishMessage>();
    let (egui_tx, egui_rx) = unbounded::<PublishMessage>();
    (
        logger_tx,
        logger_rx,
        publish_message_tx,
        publish_message_rx,
        egui_tx,
        egui_rx,
    )
}

// Aux: fn reemplazable por "?".
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

fn main() -> Result<(), Error>{
    let broker_addr = get_broker_address();

    let (logger_tx, logger_rx, publish_message_tx, publish_message_rx, egui_tx, egui_rx) =
        create_channels();

    match establish_mqtt_broker_connection(&broker_addr) {
        Ok(stream) => {
            let mut mqtt_client_listener =
                MQTTClientListener::new(stream.try_clone()?, publish_message_tx);
            let mqtt_client: MQTTClient = MQTTClient::new(stream, mqtt_client_listener.clone());
            let sistema_monitoreo = SistemaMonitoreo::new(logger_tx, egui_tx);
            let handler_1 = thread::spawn(move || {
                let _ = mqtt_client_listener.read_from_server();
            });

            let mut handlers = sistema_monitoreo.spawn_threads(
                logger_rx,
                publish_message_rx,
                egui_rx,
                mqtt_client,
            );

            handlers.push(handler_1);
            join_all_threads(handlers);
        }
        Err(e) => println!(
            "Error al establecer la conexión con el broker MQTT: {:?}",
            e
        ),
    }

    Ok(())
}
