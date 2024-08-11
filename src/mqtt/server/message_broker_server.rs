use rustx::logging::string_logger::StringLogger;
use rustx::mqtt::server::mqtt_server::MQTTServer;
use std::env::args;
use std::io::{Error, ErrorKind};

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
    let (ip, port) = load_port()?;

    // Se crean y configuran ambos extremos del string logger
    let (mut logger, handle_logger) = StringLogger::create_logger(get_formatted_app_id());

    let mqtt_server = MQTTServer::new(logger.clone_ref());
    mqtt_server.run(ip, port)?;

    // Se cierra el logger
    logger.stop_logging();
    drop(mqtt_server);

    // Se espera al hijo para el logger
    if handle_logger.join().is_err() {
        println!("Error al esperar al hijo para string logger writer.")
    }

    Ok(())
}

fn get_formatted_app_id() -> String {
    String::from("Server.")
}
