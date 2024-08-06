use std::io::Error;

use rustx::apps::{
    apps_mqtt_topics::AppsMqttTopics,
    common_clients::{get_app_will_topic, join_all_threads},
    sist_dron::{dron::Dron, utils::get_id_lat_long_and_broker_address},
};
use rustx::logging::string_logger::StringLogger;
use rustx::mqtt::client::mqtt_client::MQTTClient;
use rustx::mqtt::mqtt_utils::will_message_utils::will_message::WillMessageData;
use rustx::mqtt::mqtt_utils::will_message_utils::{app_type::AppType, will_content::WillContent};

fn get_formatted_app_id(id: u8) -> String {
    format!("dron-{}", id)
}

fn get_app_will_msg_content(id: u8) -> WillContent {
    WillContent::new(AppType::Dron, Some(id))
}

fn main() -> Result<(), Error> {
    let (id, lat, lon, broker_addr) = get_id_lat_long_and_broker_address()?;

    // Se crean y configuran ambos extremos del string logger
    let (logger, handle_logger) = StringLogger::create_logger(get_formatted_app_id(id));

    // Se inicializa la conexión mqtt y el dron
    let qos = 1; // []
    let client_id = get_formatted_app_id(id);
    let will_msg_content = get_app_will_msg_content(id);
    let will_msg_data =
        WillMessageData::new(will_msg_content.to_str(), get_app_will_topic(), qos, 1);
    match MQTTClient::mqtt_connect_to_broker(client_id, &broker_addr, Some(will_msg_data)) {
        Ok((mut mqtt_client, publish_message_rx, handle)) => {
            println!("Conectado al broker MQTT.");
            logger.log("Conectado al broker MQTT".to_string());

            let mut dron = Dron::new(id, lat, lon, logger)?; //

            //match dron_res {
            //Ok(mut dron) => {
            //let mqtt_client_ref = Arc::new(Mutex::new(mqtt_client)); // Aux: quise agregar esto para llamar a
            //dron.publish_current_info(&Arc::clone(&mqtt_client_ref)); // aux: a esta función, pero es meterse en líos de borrow.
            if let Ok(ci) = &dron.get_current_info().lock() {
                // Aux: dejo esto como estaba. Capaz qe se haga internamente? Ver [].
                mqtt_client.mqtt_publish(
                    AppsMqttTopics::DronTopic.to_str(),
                    &ci.to_bytes(),
                    dron.get_qos(),
                )?;
            };

            let mut handles = dron.spawn_threads(mqtt_client, publish_message_rx)?;

            handles.push(handle);

            join_all_threads(handles);
            //}
            //}
        }
        Err(e) => println!("Dron ID {} : Error al conectar al broker MQTT: {:?}", id, e),
    }

    // Se espera al hijo para el logger writer
    if handle_logger.join().is_err() {
        println!("Error al esperar al hijo para string logger writer.")
    }

    Ok(())
}
