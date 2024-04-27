use log::{info, warn, error};
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};

fn handle_client(mut stream: TcpStream) {
    let mut buf = [0; 1024];
    let _ = stream.read(&mut buf).expect("Error al leer mensaje");

    let username = "sistema-monitoreo";
    let password = "rutx123";
    let is_authentic = buf.windows(username.len() + password.len() + 2).any(|slice| {
        slice == [username.as_bytes(), password.as_bytes(), &[0x00]].concat()
    });

    let connack_response: [u8; 4] = if is_authentic {
        [0x20, 0x02, 0x00, 0x00] // CONNACK (0x20) con retorno 0x00
    } else {
        [0x20, 0x02, 0x00, 0x05] // CONNACK (0x20) con retorno 0x05 (Refused, not authorized)
    };
    stream.write_all(&connack_response).expect("Error al enviar CONNACK");
}

fn main() {
    env_logger::init();

    info!("Este es un mensaje de información.");
    warn!("Esto es una advertencia.");
    error!("Esto es un error crítico.");

    let listener = TcpListener::bind("127.0.0.1:9090").expect("Error al enlazar el puerto");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(|| {
                    handle_client(stream);
                });
            }
            Err(e) => {
                println!("Error al aceptar la conexión: {}", e);
            }
        }
    }
}
