use std::{io::Error, net::SocketAddr, thread::JoinHandle};

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
    for hijo in children {
        if let Err(e) = hijo.join() {
            eprintln!("Error al esperar el hilo: {:?}", e);
        }
    }
}