use config::{Config, File, FileFormat};
use log::{error, info};

use rustx::connect_message::ConnectMessage;
use rustx::mqtt_client::MQTTClient;
use rustx::subscribe_message::{SubscribeMessage, subs_msg_from_bytes};
// Este archivo representa a un cliente cualquiera. Así usará cada cliente a la librería MQTT.

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
    let mut connect_msg = ConnectMessage::new(
        0x01 << 4, // Me fijé y el fixed header no estaba shifteado, el message type tiene que quedar en los 4 bits más signifs del primer byte (toDo: arreglarlo para el futuro)
        // toDo: obs: además, al propio new podría agregarlo, no? para no tener yo que acordarme qué tipo es cada mensaje.
        "rust-client",
        None, // will_topic
        None, // will_message
        Some("sistema-monitoreo"),
        Some("rustx123"),
    );
   
    let mqtt_client_res = MQTTClient::connect_to_broker(&broker_addr, &mut connect_msg);
    match mqtt_client_res {
        Ok(mut mqtt_client) => {
            info!("Conectado al broker MQTT.");

            // publish
            let res = mqtt_client.mqtt_publish("topic3", "hola mundo :)".as_bytes());
            match res {
                Ok(_) => println!("Hecho un publish exitosamente"),
                Err(e) => {
                    error!("Error al hacer el publish {:?}", e);
                    println!("------------------------------------");
                },
            }
        },
        Err(e) => error!("Error al conectar al broker MQTT: {:?}", e),
    }


    // Construyo subscribe
    let packet_id: u16 = 1;
    let topics_to_subscribe: Vec<(String, u8)> = vec![(String::from("topic1"),1)];
    let subscribe_msg = SubscribeMessage::new(packet_id, topics_to_subscribe);
    let subs_bytes = subscribe_msg.to_bytes();
    println!("Enviando mensaje {:?}", subscribe_msg);

    let _msg_reconstruido = subs_msg_from_bytes(subs_bytes);
    // enviarlo, etc.

    

}
