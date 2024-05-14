use std::io::Read;
use std::thread;

use config::{Config, File, FileFormat};
use log::{error, info};

use rustx::connect_message::ConnectMessage;
use rustx::mqtt_client::MQTTClient;
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

    // Cliente usa funciones connect, publish, y subscribe de la lib.
    let mqtt_client_res = MQTTClient::connect_to_broker(&broker_addr, &mut connect_msg);
    match mqtt_client_res {
        Ok(mut mqtt_client) => {
            //info!("Conectado al broker MQTT."); // 
            println!("Cliente: Conectado al broker MQTT.");

            /*// Cliente usa subscribe // [] Consulta
            let res_sub = mqtt_client.mqtt_subscribe(1, vec![(String::from("topic1"), 1)]);
            match res_sub {
                Ok(_) => {println!("Cliente: Hecho un subscribe exitosamente");},
                Err(e) => {println!("Cliente: Error al hacer un subscribe: {:?}", e);},
            }

            // Cliente usa publish
            let res = mqtt_client.mqtt_publish("topic3", "hola mundo :)".as_bytes());
            match res {
                Ok(_) => {
                    println!("Cliente: Hecho un publish exitosamente");
                },
                Err(e) => {
                    error!("Cliente: Error al hacer el publish {:?}", e);
                    println!("------------------------------------");
                }
            }*/

            let mut mqtt_client_para_hijo = mqtt_client.clone(); // [] ?? ver
            let mut mqtt_client_una_ref_extra = mqtt_client.clone(); // 
            let h = thread::spawn(move ||{
                // Cliente usa publish
                let res = mqtt_client_para_hijo.mqtt_publish("topic3", "hola mundo :)".as_bytes());
                match res {
                    Ok(_) => {
                        println!("Cliente: Hecho un publish exitosamente");
                        
                    },
                    Err(e) => {
                        error!("Cliente: Error al hacer el publish {:?}", e);
                        println!("------------------------------------");
                    }
                }
            });

            let h2 = thread::spawn(move || {
                // Cliente usa subscribe // [] Consulta
                let res_sub = mqtt_client.mqtt_subscribe(1, vec![(String::from("topic1"), 1)]);
                match res_sub {
                    Ok(_) => {println!("Cliente: Hecho un subscribe exitosamente");},
                    Err(e) => {println!("Cliente: Error al hacer un subscribe: {:?}", e);},
                }
                // Probando (igual el error ya ocurrió antes de llegar hasta acá)
                let stream = mqtt_client.get_stream();
                let mut veces = 3;
                while veces > 0 {
                    println!("[loop cliente subscribe] vuelta por intentar leer");
                    // Leo la respuesta
                    let mut bytes_leidos = [0; 6]; // [] Aux temp: 6 para 1 elem, 8 p 2, 10 p 3, en realidad hay que leer el fixed hdr como en server.
                    {
                        let mut s = stream.lock().unwrap();
                        let _cant_leida = s.read(&mut bytes_leidos).unwrap();
                    }
                    println!("[loop cliente subscribe] vuelta leí bytes: {:?}", bytes_leidos);
                    veces -= 1;    
                }
            });

            /*// Cliente usa subscribe // [] Consulta
            let res_sub = mqtt_client.mqtt_subscribe(1, vec![(String::from("topic1"), 1)]);
            match res_sub {
                Ok(_) => {println!("Cliente: Hecho un subscribe exitosamente");},
                Err(e) => {println!("Cliente: Error al hacer un subscribe: {:?}", e);},
            }*/


            if h.join().is_err() {
                println!("Error al esperar a hijo.");
            }
            if h2.join().is_err() {
                println!("Error al esperar a hijo.");
            }

        }
        Err(e) => error!("Cliente: Error al conectar al broker MQTT: {:?}", e),
    }
}
