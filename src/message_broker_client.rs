use log::{info, warn, error};
use rustx::mqtt_client::MQTTClient;
use rustx::connect_message::ConnectMessage;


fn main() {
    env_logger::init();

    info!("Este es un mensaje de información.");
    warn!("Esto es una advertencia.");
    error!("Esto es un error crítico.");

    let broker_addr = "127.0.0.1:9090".parse().expect("Dirección no válida");
    let connect_msg = ConnectMessage::new("rust-client", true, 10, Some("sistema-monitoreo"), Some("rustx123"));
    let mqtt_client = MQTTClient::new();

    mqtt_client.unwrap().connect_to_broker(&broker_addr, &connect_msg);

}

