use config::{Config, File, FileFormat};
use log::{error, info};

use rustx::connect_message::ConnectMessage;
use rustx::mqtt_client::MQTTClient;
use rustx::subscribe_message::SubscribeMessage;

fn main() {
    env_logger::init();

    info!("Leyendo Archivo de Configuración");
    let mut config = Config::default();
    config
        .merge(File::new(
            "message_broker_client_config.properties",
            FileFormat::Toml,
        ))
        .unwrap();

    let ip = config
        .get::<String>("ip")
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = config.get::<u16>("port").unwrap_or(9090);
    let broker_addr = format!("{}:{}", ip, port)
        .parse()
        .expect("Dirección no válida");
    let connect_msg = ConnectMessage::new(
        "rust-client",
        true,
        10,
        Some("sistema-monitoreo"),
        Some("rustx123"),
    );

    match MQTTClient::connect_to_broker(&broker_addr, &connect_msg) {
        Ok(_) => info!("Conectado al broker MQTT."),
        Err(e) => error!("Error al conectar al broker MQTT: {:?}", e),
    }


    // Construyo subscribe
    let packet_id: u16 = 1;
    let topics_to_subscribe: Vec<(String, u8)> = vec![(String::from("topic1"),1)];
    let subscribe_msg = SubscribeMessage{ packet_type: 8, flags: 2, packet_identifier: packet_id, topic_filters: topics_to_subscribe };
    let subs_bytes = subscribe_msg.to_bytes();
    println!("Enviando mensaje {:?}", subscribe_msg);
    

}
