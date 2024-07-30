use std::sync::mpsc;

use std::io::Error;

use rustx::apps::{
    common_clients::{get_broker_address, join_all_threads},
    sist_camaras::{manage_stored_cameras::create_cameras, sistema_camaras::SistemaCamaras},
};
use rustx::logging::string_logger::StringLogger;
use rustx::mqtt::client::mqtt_client::MQTTClient;
use rustx::mqtt::mqtt_utils::will_message_utils::{app_type::AppType, will_content::WillContent};

type Channels = (
    mpsc::Sender<Vec<u8>>,
    mpsc::Receiver<Vec<u8>>,
    mpsc::Sender<bool>,
    mpsc::Receiver<bool>,
);

fn create_channels() -> Channels {
    let (cameras_tx, cameras_rx) = mpsc::channel::<Vec<u8>>();
    let (exit_tx, exit_rx) = mpsc::channel::<bool>();
    (cameras_tx, cameras_rx, exit_tx, exit_rx)
}

fn get_formatted_app_id() -> String {
    String::from("Sistema-Camaras")
}

fn get_app_will_msg_content() -> WillContent {
    WillContent::new(AppType::Cameras, 0)
}

fn main() -> Result<(), Error> {
    let broker_addr = get_broker_address();
    let (cameras_tx, cameras_rx, exit_tx, exit_rx) = create_channels();
    let cameras = create_cameras();

    // Se crean y configuran ambos extremos del string logger
    let (logger, handle_logger) = StringLogger::create_logger(get_formatted_app_id());

    let qos = 1; // []
    let client_id = get_formatted_app_id();
    let will_msg_content = get_app_will_msg_content();
    match MQTTClient::mqtt_connect_to_broker(
        client_id.as_str(),
        &broker_addr,
        will_msg_content,
        rustx::apps::apps_mqtt_topics::AppsMqttTopics::DescTopic.to_str(),
        qos,
    ) {
        Ok((mqtt_client, publish_message_rx, handle)) => {
            println!("Cliente: Conectado al broker MQTT.");

            let mut sistema_camaras = SistemaCamaras::new(cameras_tx, exit_tx, cameras, logger);

            let mut handlers =
                sistema_camaras.spawn_threads(cameras_rx, exit_rx, publish_message_rx, mqtt_client);

            handlers.push(handle);
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
