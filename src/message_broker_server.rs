use config::{Config, File, FileFormat};
use log::info;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn handle_client(mut stream: TcpStream) {
    let mut buf = [0; 1024];
    let _ = stream.read(&mut buf).expect("Error al leer mensaje");

    let username = "sistema-monitoreo";
    let password = "rutx123";
    let is_authentic: bool = buf
        .windows(username.len() + password.len() + 2)
        .any(|slice| slice == [username.as_bytes(), password.as_bytes(), &[0x00]].concat());

    let connack_response: [u8; 4] = if is_authentic {
        [0x20, 0x02, 0x00, 0x00] // CONNACK (0x20) con retorno 0x00
    } else {
        [0x20, 0x02, 0x00, 0x05] // CONNACK (0x20) con retorno 0x05 (Refused, not authorized)
    };
    stream
        .write_all(&connack_response)
        .expect("Error al enviar CONNACK");

    // 
    stream.read(&mut buf).unwrap(); // Aux probando []
    println!("DESDE SERVER, LEO: {:?}", buf); // (veo manualmente que sí me llegó el struct), en construcción
}

fn main() {
    env_logger::init();

    info!("Leyendo archivo de configuración.");
    let mut config = Config::default();
    config
        .merge(File::new(
            "message_broker_server_config.properties",
            FileFormat::Toml,
        ))
        .unwrap();

    let ip = config
        .get::<String>("ip")
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = config.get::<u16>("port").unwrap_or(9090);
    let listener =
        TcpListener::bind(format!("{}:{}", ip, port)).expect("Error al enlazar el puerto");

    let mut handles = vec![];

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let handle = std::thread::spawn(|| {
                    handle_client(stream);
                });
                handles.push(handle);
            }
            Err(e) => {
                println!("Error al aceptar la conexión: {}", e);
            }
        }
    }

    for handle in handles {
        if let Err(e) = handle.join() {
            eprintln!("Error al esperar el hilo: {:?}", e);
        }
    }
}