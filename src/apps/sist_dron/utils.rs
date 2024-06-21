// Archivo con funciones.
// [] Las funciones de este archivo devuelven un Result.

use std::{
    io::{Error, ErrorKind},
    net::SocketAddr,
};

/// Lee y devuelve, de los argumentos ingresados al correr el programa,
/// el id del dron, y la IP y el puerto del servidor al que el cliente se va a conectar.
fn load_id_ip_and_port() -> Result<(u8, String, u16), Error> {
    let argv = std::env::args().collect::<Vec<String>>();
    if argv.len() != 4 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Cantidad de argumentos inválida. Debe ingresar la dirección IP y 
        el puerto del servidor.",
        ));
    }

    let id = argv[1]
        .parse::<u8>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "El id proporcionado no es válido"))?;
    let ip = &argv[2];
    let port = argv[3].parse::<u16>().map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            "El puerto proporcionado no es válido",
        )
    })?;

    Ok((id, ip.to_string(), port))
}

/// Construye y devuelve la broker_address necesaria para conectarse al servidor mqtt,
/// a partir de los argumentos recibidos de ip y puerto.
pub fn get_id_and_broker_address() -> Result<(u8, SocketAddr), Error> {
    let (id, ip, port) = load_id_ip_and_port()?;
    let addr: String = format!("{}:{}", ip, port);
    let broker_addr = addr
        .parse()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Dirección no válida"))?;

    Ok((id, broker_addr))
}

// Función no usada al menos por ahora
// pub fn join_all_threads(children: Vec<JoinHandle<()>>) {
//     for child in children {
//         if let Err(e) = child.join() {
//             eprintln!("Error al esperar el hilo: {:?}", e);
//         }
//     }
// }
