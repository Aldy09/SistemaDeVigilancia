use std::{io::Error, sync::mpsc, thread};

use rustx::{
    apps::{
        apps_mqtt_topics::AppsMqttTopics,
        common_clients::join_all_threads,
        sist_dron::{dron::Dron, utils::get_id_lat_long_and_broker_address},
    },
    logging::string_logger::StringLogger,
    mqtt::{
        client::{
            mqtt_client::MQTTClient, mqtt_client_listener::MQTTClientListener,
            mqtt_client_server_connection::mqtt_connect_to_broker,
        },
        messages::publish_message::PublishMessage, mqtt_utils::will_message_utils::{app_type::AppType, will_content::WillContent},
    },
};

type Channels = (mpsc::Sender<PublishMessage>, mpsc::Receiver<PublishMessage>);

fn create_channels() -> Channels {
    let (publish_message_tx, publish_message_rx) = mpsc::channel::<PublishMessage>();
    (publish_message_tx, publish_message_rx)
}

fn get_formatted_app_id(id: u8) -> String {
    format!("dron-{}", id)
}

fn get_app_will_msg_content(id: u8) -> WillContent {
    WillContent::new(AppType::Dron, id)
}

fn main() -> Result<(), Error> {
    let (id, lat, lon, broker_addr): (u8, f64, f64, std::net::SocketAddr) =
        get_id_lat_long_and_broker_address()?;

    let (publish_message_tx, publish_message_rx) = create_channels();

    // Se crean y configuran ambos extremos del string logger
    let (logger, handle_logger) = StringLogger::create_logger(get_formatted_app_id(id));

    // Se inicializa la conexión mqtt y el dron
    let qos = 1; // []
    let client_id = get_formatted_app_id(id);
    let will_msg_content = get_app_will_msg_content(id);
    match MQTTClient::mqtt_connect_to_broker(client_id.as_str(), &broker_addr, will_msg_content, rustx::apps::apps_mqtt_topics::AppsMqttTopics::DescTopic.to_str(), qos) {
        Ok((mut mqtt_client, publish_message_rx, handle)) => {            
            println!("Cliente: Conectado al broker MQTT.");

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

            let mut handlers = dron.spawn_threads(mqtt_client, publish_message_rx)?;

            handlers.push(handle);

            join_all_threads(handlers);
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
