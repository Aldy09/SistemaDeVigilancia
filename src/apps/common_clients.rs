use std::{
    io::Error,
    net::SocketAddr,
    sync::{mpsc::Receiver, Arc, Mutex},
    thread::JoinHandle,
};

use crate::mqtt::client::mqtt_client::MQTTClient;

/// Lee el IP del cliente y el puerto en el que el cliente se va a conectar al servidor.
fn load_ip_and_port() -> Result<(String, u16), Box<Error>> {
    let argv = std::env::args().collect::<Vec<String>>();
    if argv.len() != 3 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Cantidad de argumentos inválido. Debe ingresar: la dirección IP y 
        el puerto en el que desea correr el servidor.",
        )));
    }
    let ip = &argv[1];
    let port = match argv[2].parse::<u16>() {
        Ok(port) => port,
        Err(_) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "El puerto proporcionado no es válido",
            )))
        }
    };

    Ok((ip.to_string(), port))
}

pub fn get_broker_address() -> SocketAddr {
    let (ip, port) = load_ip_and_port().unwrap_or_else(|e| {
        println!("Error al cargar el puerto: {:?}", e);
        std::process::exit(1);
    });

    let broker_addr: String = format!("{}:{}", ip, port);
    broker_addr.parse().expect("Dirección no válida")
}

pub fn join_all_threads(children: Vec<JoinHandle<()>>) {
    for child in children {
        if let Err(e) = child.join() {
            eprintln!("Error al esperar el hilo: {:?}", e);
        }
    }
}

/// Función a llamar desde un hilo dedicado, para que app escuche si dicha app desea salir.
/// Al recibir por el rx, se encarga de enviar disconnect de mqtt.
pub fn exit_when_asked(mqtt_client: Arc<Mutex<MQTTClient>>, exit_rx: Receiver<bool>) {
    // Espero que otro hilo (ej la ui, ej el abm) me indique que se desea salir
    let exit_res = exit_rx.recv();
    match exit_res {
        Ok(exit) => {
            // Cuando eso ocurre, envío disconnect por mqtt
            if exit {
                if let Ok(mut mqtt_locked) = mqtt_client.lock() {
                    match mqtt_locked.mqtt_disconnect() {
                        Ok(_) => println!("Saliendo exitosamente."),
                        Err(e) => println!("Error al salir: {:?}", e),
                    }
                }

                // Aux: ver si hay que hacer algo más para salir [].
            }
        }
        Err(e) => println!("Error al recibir por exit_rx {:?}", e),
    }
}

/// Devuelve true si la app de cliente debe dejar de loopear para leer de mqtt.
pub fn is_disconnected_error() {
    println!("Cliente: No hay más PublishMessage's por leer.");
}
