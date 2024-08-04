use rustx::mqtt::server::incoming_connections::ClientListener;
use rustx::mqtt::server::mqtt_server_2::MQTTServer;
use std::env::args;
use std::io::{Error, ErrorKind};
use std::net::TcpListener;
use std::thread;

//use rustx::mqtt_server::{create_server, handle_incoming_connections, load_port};

/// Lee el puerto por la consola, y devuelve la dirección IP y el puerto.
pub fn load_port() -> Result<(String, u16), Error> {
    let argv = args().collect::<Vec<String>>();
    if argv.len() != 2 {
        return Err(Error::new(ErrorKind::InvalidInput, "Cantidad de argumentos inválido. Debe ingresar el puerto en el que desea correr el servidor."));
    }
    let port = match argv[1].parse::<u16>() {
        Ok(port) => port,
        Err(_) => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "El puerto proporcionado no es válido",
            ))
        }
    };
    let localhost = "127.0.0.1".to_string();

    Ok((localhost, port))
}

/// Crea un servidor en la dirección ip y puerto especificados.
fn create_server(ip: String, port: u16) -> Result<TcpListener, Error> {
    let listener =
        TcpListener::bind(format!("{}:{}", ip, port)).expect("Error al enlazar el puerto");
    Ok(listener)
}

fn main() -> Result<(), Error> {
    let (ip, port) = load_port()?;
    let listener = create_server(ip, port)?;

    let mqtt_server = MQTTServer::new()?;
    let mut incoming_connections = ClientListener::new();

    // Hilo para manejar las conexiones entrantes
    let thread_incoming = thread::spawn(move || {
        let _ = incoming_connections.handle_incoming_connections(listener, mqtt_server);
    });

    thread_incoming.join().unwrap();

    Ok(())
}
