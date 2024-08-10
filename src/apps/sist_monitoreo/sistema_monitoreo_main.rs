use std::io::Error;

use rustx::apps::{
    common_clients::{get_broker_address, join_all_threads},
    sist_monitoreo::sistema_monitoreo::SistemaMonitoreo,
};
use rustx::logging::string_logger::StringLogger;
use rustx::mqtt::client::mqtt_client::MQTTClient;

fn get_formatted_app_id() -> String {
    String::from("Sistema-Monitoreo")
}

fn main() -> Result<(), Error> {
    let broker_addr = get_broker_address();

    // Se crean y configuran ambos extremos del string logger
    let (mut logger, handle_logger) = StringLogger::create_logger(get_formatted_app_id());

    let client_id = get_formatted_app_id();
    let sistema_monitoreo = SistemaMonitoreo::new(logger.clone_ref());
    match MQTTClient::mqtt_connect_to_broker(client_id, &broker_addr, None) {
        Ok((mqtt_client, publish_message_rx, handle)) => {
            println!("Conectado al broker MQTT.");
            logger.log("Conectado al broker MQTT".to_string());

            let mut handles = sistema_monitoreo.spawn_threads(publish_message_rx, mqtt_client);

            handles.push(handle);
            join_all_threads(handles);

        }
        Err(e) => println!(
            "Sistema-Monitoreo: Error al conectar al broker MQTT: {:?}",
            e
        ),
    }
    logger.stop_logging();
    drop(sistema_monitoreo); // porque le hicimos clone_ref al logger.

    // Se espera al hijo para el logger writer
    if handle_logger.join().is_err() {
        println!("Error al esperar al hijo para string logger writer.")
    }

    Ok(())
}
