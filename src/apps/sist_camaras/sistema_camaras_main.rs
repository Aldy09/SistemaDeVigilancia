use std::io::Error;

use rustx::logging::string_logger::StringLogger;
use rustx::mqtt::mqtt_utils::will_message_utils::will_message::WillMessageData;
use rustx::mqtt::mqtt_utils::will_message_utils::{app_type::AppType, will_content::WillContent};
use rustx::{
    apps::{
        common_clients::{get_app_will_topic, get_broker_address, join_all_threads},
        sist_camaras::{manage_stored_cameras::create_cameras, sistema_camaras::SistemaCamaras},
    },
    mqtt::client::mqtt_client::MQTTClient,
};

fn get_formatted_app_id() -> String {
    String::from("Sistema-Camaras")
}

fn get_app_will_msg_content() -> WillContent {
    WillContent::new(AppType::Cameras, None)
}

fn main() -> Result<(), Error> {
    let broker_addr = get_broker_address();
    let cameras = create_cameras();

    // Se crean y configuran ambos extremos del string logger
    let (mut logger, handle_logger) = StringLogger::create_logger(get_formatted_app_id());

    let qos = 1; // []
    let client_id = get_formatted_app_id();
    let will_msg_content = get_app_will_msg_content();
    let will_msg_data =
        WillMessageData::new(will_msg_content.to_str(), get_app_will_topic(), qos, 1);
    match MQTTClient::mqtt_connect_to_broker(client_id, &broker_addr, Some(will_msg_data)) {
        Ok((mqtt_client, publish_msg_rx, handle)) => {
            println!("Conectado al broker MQTT.");
            logger.log("Conectado al broker MQTT".to_string());

            let mut sistema_camaras = SistemaCamaras::new(cameras, logger.clone_ref());

            let mut handles = sistema_camaras.spawn_threads(publish_msg_rx, mqtt_client);

            handles.push(handle);
            join_all_threads(handles);
        }
        Err(e) => println!("Error al conectar al broker MQTT: {:?}", e),
    }

    logger.stop_logging();

    println!("Hola.");

    // Se espera al hijo para el logger
    if handle_logger.join().is_err() {
        println!("Error al esperar al hijo para string logger writer.")
    }
    
    println!("Hola.");

    Ok(())
}
