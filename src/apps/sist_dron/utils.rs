// Archivo con funciones.
// [] Las funciones de este archivo devuelven un Result.

use std::{
    io::{Error, ErrorKind},
    net::SocketAddr,
};

/// Lee y devuelve, de los argumentos ingresados al correr el programa,
/// el id del dron, y la IP y el puerto del servidor al que el cliente se va a conectar.
fn load_id_lat_long_ip_and_port() -> Result<(u8, f64, f64, String, u16), Error> {
    let argv = std::env::args().collect::<Vec<String>>();
    if argv.len() != 6 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Cantidad de argumentos inválida. Debe ingresar el ID, latitud, longitud, dirección IP y el puerto del servidor.",
        ));
    }

    let id = argv[1]
        .parse::<u8>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "El id proporcionado no es válido"))?;
    let latitud = argv[2]
        .parse::<f64>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "La latitud proporcionada no es válida"))?;
    let longitud = argv[3]
        .parse::<f64>()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "La longitud proporcionada no es válida"))?;
    
    let ip = &argv[4];
    let port = argv[5].parse::<u16>().map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            "El puerto proporcionado no es válido",
        )
    })?;

    Ok((id, latitud, longitud, ip.to_string(), port))
}

/// Construye y devuelve la broker_address necesaria para conectarse al servidor mqtt,
/// a partir de los argumentos recibidos de id, latitud, longitud, ip y puerto.
/// También devuelve la latitud y longitud.
pub fn get_id_lat_long_and_broker_address() -> Result<(u8, f64, f64, SocketAddr), Error> {
    let (id, latitud, longitud, ip, puerto) = load_id_lat_long_ip_and_port()?;
    let addr: String = format!("{}:{}", ip, puerto);
    let broker_addr = addr
        .parse()
        .map_err(|_| Error::new(ErrorKind::InvalidInput, "Dirección no válida"))?;

    Ok((id, latitud, longitud, broker_addr))
}

// Función no usada al menos por ahora
// pub fn join_all_threads(children: Vec<JoinHandle<()>>) {
//     for child in children {
//         if let Err(e) = child.join() {
//             eprintln!("Error al esperar el hilo: {:?}", e);
//         }
//     }
// }
