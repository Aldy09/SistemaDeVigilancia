use rustx::mqtt_server;
use std::env::args;
use std::io::{Error, ErrorKind};

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

fn main() -> Result<(), Error> {
    env_logger::init();

    let (ip, port) = load_port()?;

    let _mqtt_server = mqtt_server::MQTTServer::new(ip, port);


    Ok(())
}
