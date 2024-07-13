use std::{
    io::Error,
    sync::mpsc,
    thread::{self, JoinHandle},
};

use rustx::{
    apps::{
        apps_mqtt_topics::AppsMqttTopics,
        common_clients::join_all_threads,
        sist_dron::{dron::Dron, utils::get_id_lat_long_and_broker_address},
    },
    logging::{
        string_logger::StringLogger, string_logger_writer::StringLoggerWriter,
        structs_to_save_in_logger::StructsToSaveInLogger,
    },
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
);

fn create_channels() -> Channels {
    let (logger_tx, logger_rx) = mpsc::channel::<StructsToSaveInLogger>();
    let (publish_message_tx, publish_message_rx) = mpsc::channel::<PublishMessage>();
    (logger_tx, logger_rx, publish_message_tx, publish_message_rx)
}

fn main() -> Result<(), Error> {
    let (id, lat, lon, broker_addr): (u8, f64, f64, std::net::SocketAddr) = get_id_lat_long_and_broker_address()?;

    // Los logger_tx y logger_rx de este tipo de datos, podrían eliminarse por ser reemplazados por el nuevo string logger; se conservan temporalmente por compatibilidad hacia atrás.
    let (logger_tx, logger_rx, publish_message_tx, publish_message_rx) = create_channels();

    // Se crean y configuran ambos extremos del string logger
    let (string_logger_tx, string_logger_rx) = mpsc::channel::<String>();
    let logger = StringLogger::new(string_logger_tx);
    let logger_writer = StringLoggerWriter::new(string_logger_rx);
    let handle_logger = spawn_dron_stuff_to_string_logger_thread(logger_writer); //

    // Se inicializa la conexión mqtt y el dron
    let client_id = format!("dron-{}", id);
    match mqtt_connect_to_broker(client_id.as_str(), &broker_addr){
        Ok(stream) => {
            let mut mqtt_client_listener =
                MQTTClientListener::new(stream.try_clone()?, publish_message_tx);
            let mut mqtt_client: MQTTClient = MQTTClient::new(stream, mqtt_client_listener.clone());
            println!("Cliente: Conectado al broker MQTT.");

            let mut dron = Dron::new(id, lat, lon, logger_tx, logger)?; //

            //match dron_res {
                //Ok(mut dron) => {
            //let mqtt_client_ref = Arc::new(Mutex::new(mqtt_client)); // Aux: quise agregar esto para llamar a
            //dron.publish_current_info(&Arc::clone(&mqtt_client_ref)); // aux: a esta función, pero es meterse en líos de borrow.
            if let Ok(ci) = &dron.get_current_info().lock() { // Aux: dejo esto como estaba. Capaz qe se haga internamente? Ver [].
                mqtt_client.mqtt_publish(
                    AppsMqttTopics::DronTopic.to_str(),
                    &ci.to_bytes(),
                    dron.get_qos(),
                )?;
            };

            let handler_1 = thread::spawn(move || { // []
                let _ = mqtt_client_listener.read_from_server();
            });

            let mut handlers =
                dron.spawn_threads(mqtt_client, publish_message_rx, logger_rx)?;

            handlers.push(handler_1);

            join_all_threads(handlers);
                //}
            //}
        }
        Err(e) => println!(
            "Dron ID {} : Error al conectar al broker MQTT: {:?}",
            id, e
        ),
    }

    // Se espera al hijo para el logger writer
    if handle_logger.join().is_err() {
        println!("Error al esperar al hijo para string logger writer.")
    }

    Ok(())
}

fn spawn_dron_stuff_to_string_logger_thread(
    string_logger_writer: StringLoggerWriter,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(msg) = string_logger_writer.logger_rx.recv() {
            if string_logger_writer.write_to_file(msg).is_err() {
                println!("LoggerWriter: error al escribir al archivo de log.");
            }
        }
    })
}
