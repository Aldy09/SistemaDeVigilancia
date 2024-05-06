use config::{Config, File, FileFormat};
use log::info;
use rustx::connect_message::ConnectMessage;
use rustx::fixed_header::{FixedHeader, self};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn handle_client(mut stream: TcpStream) {

    const FIXED_HEADER_LEN: usize = FixedHeader::fixed_header_len();
    let mut fixed_header_buf: [u8; 2] = [0; FIXED_HEADER_LEN];

    let res = stream.read_exact(&mut fixed_header_buf);
    match res {
        Ok(_) => {
            // He leído bytes de un fixed_header, tengo que ver de qué tipo es.
            let fixed_header = FixedHeader::from_bytes(fixed_header_buf.to_vec());
            let tipo = fixed_header.get_tipo();
            println!("Recibo msj con tipo: {}, bytes de fixed header leidos: {:?}", tipo, fixed_header);
            match tipo {
                1 => { // Es tipo Connect, acá procesar el connect y dar connack
                    // Instancio un buffer para leer los bytes restantes, siguientes a los de fixed header
                    let msg_rem_len: usize = fixed_header.get_rem_len();
                    let mut rem_buf = vec![0; msg_rem_len];
                    let _res = stream.read_exact(&mut rem_buf).expect("Error al leer mensaje");

                    // Ahora junto las dos partes leídas, para obt mi msg original
                    let mut buf = fixed_header_buf.to_vec();
                    buf.extend(rem_buf);
                    // Entonces tengo el mensaje completo
                    let connect_msg = ConnectMessage::from_bytes(&buf);


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

                },
                3 => { // Es Publish



                },
                8 => { // Es Subscribe

                },
                _ => println!("Error, tipo de mensaje recibido no identificado."),

            }
        },
        Err(e) => println!("Error al leer el los bytes del fixed header por el socket."),
    }



    

    // 
    //stream.read(&mut buf).unwrap(); // Aux probando []
    //println!("DESDE SERVER, LEO: {:?}", buf); // (veo manualmente que sí me llegó el struct), en construcción
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