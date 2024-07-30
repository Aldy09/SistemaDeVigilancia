use std::io::Error;

use crossbeam_channel::unbounded;
use rustx::apps::{
    common_clients::{get_app_will_topic, get_broker_address, join_all_threads},
    sist_monitoreo::sistema_monitoreo::SistemaMonitoreo,
};
use rustx::logging::string_logger::StringLogger;
use rustx::mqtt::mqtt_utils::will_message_utils::{app_type::AppType, will_content::WillContent};
use rustx::mqtt::{client::mqtt_client::MQTTClient, messages::publish_message::PublishMessage};

fn get_formatted_app_id() -> String {
    String::from("Sistema-Monitoreo")
}

fn get_app_will_msg_content() -> WillContent {
    WillContent::new(AppType::Monitoreo, None)
}

fn main() -> Result<(), Error> {
    let broker_addr = get_broker_address();

    // Los logger_tx y logger_rx de este tipo de datos, podrían eliminarse por ser reemplazados por el nuevo string logger; se conservan temporalmente por compatibilidad hacia atrás.
    let (egui_tx, egui_rx) = unbounded::<PublishMessage>();

    // Se crean y configuran ambos extremos del string logger
    let (logger, handle_logger) = StringLogger::create_logger(get_formatted_app_id());

    let qos = 1; // []
    let client_id = get_formatted_app_id();
    let will_msg_content = get_app_will_msg_content();
    match MQTTClient::mqtt_connect_to_broker(
        client_id,
        &broker_addr,
        will_msg_content.to_str(),
        get_app_will_topic(),
        qos,
    ) {
        Ok((mqtt_client, publish_message_rx, handle)) => {
            println!("Conectado al broker MQTT.");
            logger.log("Conectado al broker MQTT".to_string());
            let sistema_monitoreo = SistemaMonitoreo::new(egui_tx, logger);

            let mut handles =
                sistema_monitoreo.spawn_threads(publish_message_rx, egui_rx, mqtt_client);

            handles.push(handle);
            join_all_threads(handles);
        }
        Err(e) => println!(
            "Sistema-Monitoreo: Error al conectar al broker MQTT: {:?}",
            e
        ),
    }

    // Se espera al hijo para el logger writer
    if handle_logger.join().is_err() {
        println!("Error al esperar al hijo para string logger writer.")
    }

    Ok(())
}
