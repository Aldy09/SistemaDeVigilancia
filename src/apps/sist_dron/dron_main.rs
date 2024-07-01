use std::{
    io::{Error, ErrorKind},
    net::{SocketAddr, TcpStream},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
};

use rustx::{
    apps::{
        apps_mqtt_topics::AppsMqttTopics,
        common_clients::join_all_threads,
        sist_dron::{dron::Dron, utils::get_id_and_broker_address},
    },
    logging::{string_logger::StringLogger, string_logger_writer::StringLoggerWriter, structs_to_save_in_logger::StructsToSaveInLogger},
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

/// Crea el client_id a partir de sus datos. Obtiene la broker_addr del server a la que conectarse, a partir de
/// los argumentos ingresados al llamar al main. Y llama al connect de mqtt.
pub fn establish_mqtt_broker_connection(
    client_id_u8: u8,
    broker_addr: &SocketAddr,
) -> Result<TcpStream, Error> {
    let client_id = format!("dron-{}", client_id_u8);
    let handshake_result = mqtt_connect_to_broker(client_id.as_str(), broker_addr);
    match handshake_result {
        Ok(stream) => {
            println!("Cliente: Conectado al broker MQTT.");

            Ok(stream)
        }
        Err(e) => {
            println!(
                "Dron ID {} : Error al conectar al broker MQTT: {:?}",
                client_id_u8, e
            );
            Err(e)
        }
    }
}

fn main() -> Result<(), Error> {
    let (id, broker_addr) = get_id_and_broker_address()?;

    let (logger_tx, logger_rx, publish_message_tx, publish_message_rx) = create_channels();
    
    // Se crean y configuran ambos extremos del string logger
    let (string_logger_tx, string_logger_rx) = mpsc::channel::<String>();
    let logger = StringLogger::new(string_logger_tx);
    let logger_writer = StringLoggerWriter::new(string_logger_rx);
    let handle_logger = spawn_dron_stuff_to_string_logger_thread(logger_writer); //

    // Se inicializa la conexión mqtt y el dron
    match establish_mqtt_broker_connection(id, &broker_addr) {
        Ok(stream) => {
            let mut mqtt_client_listener =
                MQTTClientListener::new(stream.try_clone().unwrap(), publish_message_tx);
            let mut mqtt_client: MQTTClient = MQTTClient::new(stream, mqtt_client_listener.clone());

            let dron_res = Dron::new(id, logger_tx, logger); //

            match dron_res {
                Ok(mut dron) => {
                    if let Ok(ci) = &dron.get_current_info().lock() {
                        mqtt_client
                            .mqtt_publish(AppsMqttTopics::DronTopic.to_str(), &ci.to_bytes())?;
                    }

                    let handler_1 = thread::spawn(move || {
                        let _ = mqtt_client_listener.read_from_server();
                    });

                    let mut handlers =
                        dron.spawn_threads(mqtt_client, publish_message_rx, logger_rx)?;

                    handlers.push(handler_1);

                    join_all_threads(handlers);
                }
                Err(_e) => {
                    return Err(Error::new(ErrorKind::Other, "Error al inicializar el dron"));
                }
            }
        }
        Err(e) => println!(
            "Error al establecer la conexión con el broker MQTT: {:?}",
            e
        ),
    }

    // Se espera al hijo para el logger writer
    if handle_logger.join().is_err() {
        println!("Error al esperar al hijo para string logger writer.")
    }

    Ok(())
}


fn spawn_dron_stuff_to_string_logger_thread(string_logger_writer: StringLoggerWriter) -> JoinHandle<()> {
    thread::spawn(move || loop {
        while let Ok(msg) = string_logger_writer.logger_rx.recv() {
            if string_logger_writer.write_to_file(msg).is_err() {
                println!("LoggerWriter: error al escribir al archivo de log.");
            }
        }
    })
}