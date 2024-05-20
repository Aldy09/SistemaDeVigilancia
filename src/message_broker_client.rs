use std::{thread, sync::{Arc, Mutex}};

use config::{Config, File, FileFormat};
use log::{error, info};

use rustx::mqtt_client::{MQTTClient, self};
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

    // Cliente usa funciones connect, publish, y subscribe de la lib.
    let mqtt_client_res = MQTTClient::connect_to_broker(&broker_addr);
    match mqtt_client_res {
        Ok(mut mqtt_client) => {
            //info!("Conectado al broker MQTT."); //
            println!("Cliente: Conectado al broker MQTT.");

            // Cliente usa subscribe
            let res_sub = mqtt_client.mqtt_subscribe(1, vec![(String::from("topic3"), 1)]);
            match res_sub {
                Ok(_) => println!("Cliente: Hecho un subscribe exitosamente"),
                Err(e) => println!("Cliente: Error al hacer un subscribe: {:?}", e),
            }

            // Cliente usa publish
            let res = mqtt_client.mqtt_publish("topic3", "hola mundo :)".as_bytes());
            match res {
                Ok(_) => println!("Cliente: Hecho un publish exitosamente"),
                Err(e) => error!("Cliente: Error al hacer el publish {:?}", e),
            }

            //  Que lea del topic al/os cual/es hizo subscribe, implementando [].
            //let mqtt_client_c = Arc::new(Mutex::new(mqtt_client)); y habrá que usar lock... podemos esperar al hijo adentro?
            let h = thread::spawn( move || {
                // while condición de corte? así puedo hacer el finalizar abajo del loop
                loop {
                    let msg_bytes = mqtt_client.mqtt_receive_msg_from_subs_topic();
                    match msg_bytes {
                        Ok(msg_b) => println!("Cliente: Recibo estos msg_bytes: {:?}", msg_b),
                        Err(e) => println!("Cliente: Error al recibir msg_bytes: {:?}", e),
                    }                    
                    // [] ToDo: aux: para que el compilador permita mandar un mensaje en vez de los bytes,
                    // tenemos que hacer un trait Message y que todos los structs de los mensajes lo implementen
                }

                // Cliente termina de utilizar mqtt
                mqtt_client.finalizar();
            });

            if h.join().is_err() {
                println!("Cliente: error al esperar a hijo que recibe msjs");
            }
        }
        Err(e) => error!("Cliente: Error al conectar al broker MQTT: {:?}", e),
    }
}
