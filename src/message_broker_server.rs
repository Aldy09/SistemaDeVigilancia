//use config::{Config, File, FileFormat};
//use log::info;
use rustx::connect_message::ConnectMessage;
use rustx::fixed_header::FixedHeader;
use rustx::puback_message::PubAckMessage;
use rustx::publish_message::PublishMessage;
use rustx::suback_message::SubAckMessage;
use rustx::subscribe_message::SubscribeMessage;
use rustx::subscribe_return_code::SubscribeReturnCode;
use std::collections::HashMap;
use std::env::args;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, self};
use std::time::Duration;

type ShareableStream = Arc<Mutex<TcpStream>>;
type ShHashmapType = Arc<Mutex<HashMap<String, Vec<ShareableStream>>>>;

/// Lee `fixed_header` bytes del `stream`, sabe cuántos son por ser de tamaño fijo el fixed_header.
/// Determina el tipo del mensaje recibido que inicia por `fixed_header`.
/// Devuelve el tipo, y por cuestiones de optimización (ahorrar conversiones)
/// devuelve también fixed_header (el struct encabezado del mensaje) y fixed_header_buf (sus bytes).
fn get_fixed_header_from_stream(
    stream: &Arc<Mutex<TcpStream>>,
) -> Result<([u8; 2], FixedHeader), Error> {
    const FIXED_HEADER_LEN: usize = FixedHeader::fixed_header_len();
    let mut fixed_header_buf: [u8; 2] = [0; FIXED_HEADER_LEN];

    // Tomo lock y leo del stream
    {
        if let Ok(mut s) = stream.lock() {
            let set_read_timeout = s.set_read_timeout(Some(Duration::from_millis(300)));
            //if set_read_timeout.is_err_and(|e| e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut) {
                match set_read_timeout {
                    Ok(_) => {
                        // Leer
                        let _res = s.read(&mut fixed_header_buf)?;
                        let _ = s.set_read_timeout(None);
                    },
                    Err(e) => {
                        if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut {
                            // Se utiliza desde afuera
                            return Err(Error::new(ErrorKind::Other, "No se leyó."));
                        } else {
                            // Rama solamente para debugging
                            println!("OTRO ERROR, que no fue de timeout: {:?}",e);
                            return Err(Error::new(ErrorKind::Other, "OTRO ERROR"));
                        }
                    },
            }
            // Else, leer
            //let _res = s.read(&mut fixed_header_buf)?;
        }
    }

    // He leído bytes de un fixed_header, tengo que ver de qué tipo es.
    let fixed_header = FixedHeader::from_bytes(fixed_header_buf.to_vec());

    Ok((fixed_header_buf, fixed_header))
}

/// Una vez leídos los dos bytes del fixed header de un mensaje desde el stream,
/// lee los siguientes `remaining length` bytes indicados en el fixed header.
/// Concatena ambos grupos de bytes leídos para conformar los bytes totales del mensaje leído.
/// (Podría hacer fixed_header.to_bytes(), se aprovecha que ya se leyó fixed_header_bytes).
fn get_message_decoded_in_bytes_from_stream(
    fixed_header: &FixedHeader,
    stream: &Arc<Mutex<TcpStream>>,
    fixed_header_bytes: &[u8; 2],
) -> Result<Vec<u8>, Error> {
    // Instancio un buffer para leer los bytes restantes, siguientes a los de fixed header
    let msg_rem_len: usize = fixed_header.get_rem_len();
    let mut rem_buf = vec![0; msg_rem_len];
    // Tomo lock y leo del stream
    {
        if let Ok(mut s) = stream.lock() {
            // si uso un if let, no nec el scope de afuera para dropear []
            let _res = s.read(&mut rem_buf)?;
        }
    }
    // Ahora junto las dos partes leídas, para obt mi msg original
    let mut buf = fixed_header_bytes.to_vec();
    buf.extend(rem_buf);
    /*println!(
        "   Mensaje en bytes recibido, antes de hacerle from_bytes: {:?}",
        buf
    );*/
    Ok(buf)
}

fn process_connect(
    fixed_header: &FixedHeader,
    stream: &Arc<Mutex<TcpStream>>,
    fixed_header_buf: &[u8; 2],
    subs_by_topic: &ShHashmapType,
) -> Result<(), Error> {
    // Continúa leyendo y reconstruye el mensaje recibido completo
    println!("Recibo mensaje tipo Connect");
    let msg_bytes =
        get_message_decoded_in_bytes_from_stream(fixed_header, stream, fixed_header_buf)?;
    let connect_msg = ConnectMessage::from_bytes(&msg_bytes);
    println!(
        "   Mensaje connect completo recibido: \n   {:?}",
        connect_msg
    );

    // Procesa el mensaje connect
    let (is_authentic, connack_response) = authenticate(connect_msg)?;

    write_to_the_client(&connack_response, stream)?;
    println!(
        "   tipo connect: Enviado el ack: {:?}",
        connack_response
    );

    if is_authentic {
        handle_connection(stream, subs_by_topic)?;
    } else {
        println!("   ERROR: No se pudo autenticar al cliente.");
    }

    Ok(())
}

fn authenticate(connect_msg: ConnectMessage) -> Result<(bool, [u8; 4]), Error> {
    let username = "sistema-monitoreo";
    let password = "rustx123";

    let mut is_authentic: bool = false;
    if let Some(msg_user) = connect_msg.get_user() {
        if let Some(msg_passwd) = connect_msg.get_passwd() {
            is_authentic = msg_user == username && msg_passwd == password;
        }
    }

    let connack_response: [u8; 4] = if is_authentic {
        [0x20, 0x02, 0x00, 0x00] // CONNACK (0x20) con retorno 0x00
    } else {
        [0x20, 0x02, 0x00, 0x05] // CONNACK (0x20) con retorno 0x05 (Refused, not authorized)
    };
    Ok((is_authentic, connack_response))
}

// A partir de ahora que ya se hizo el connect exitosamente,
// se puede empezar a recibir publish y subscribe de ese cliente.
// Cono un mismo cliente puede enviarme múltiples mensajes, no solamente uno, va un loop.               14,15,45,451548,4,4,445,
// Leo, y le paso lo leído a la función hasta que lea [0, 0].
fn handle_connection(
    stream: &Arc<Mutex<TcpStream>>,
    subs_by_topic: &ShHashmapType,
) -> Result<(), Error> {
    // buffer, fixed_header, tipo
    println!("Server esperando mensajes.");
    let mut fixed_header_info = get_fixed_header_from_stream(stream)?;
    let ceros: &[u8; 2] = &[0; 2];
    //let ceros: &[u8; 2] = &[32+64+128; 2];
    let mut vacio = &fixed_header_info.0 == ceros;
    while !vacio {
        continue_with_conection(stream, subs_by_topic, &fixed_header_info)?; // esta función lee UN mensaje.
                                                                            // Leo para la siguiente iteración
        //fixed_header_info = get_fixed_header_from_stream(stream)?;
        //vacio = &fixed_header_info.0 == ceros;
        /*match get_fixed_header_from_stream(stream){
            Ok(_) => {
                println!("While: leí bien.");
                vacio = &fixed_header_info.0 == ceros;

            },
            Err(_) => {
                println!("While: leí error de timeout.");
                // Acá me gustaría volver a leer, pero hasta cuándo.
                // Es un loop. Cuando pudo leer sin error, hace break.
            },
        }*/
        println!("Server esperando más mensajes.");
        loop {
            match get_fixed_header_from_stream(stream){
                Ok((fixed_h, fixed_h_buf)) => {
                    println!("While: leí bien.");
                    // Guardo lo leído y comparo para siguiente vuelta del while
                    fixed_header_info = (fixed_h, fixed_h_buf);
                    vacio = &fixed_header_info.0 == ceros;
                    break;
    
                },
                Err(_) => {
                    //println!("While: leí error de timeout."); // no quiero printear, se llena de prints
                    // Acá me gustaría volver a leer, pero hasta cuándo.
                    // Es un loop. Cuando pudo leer sin error, hace break.
                },
            };
            thread::sleep(Duration::from_millis(300));
        };
        println!("Server aux: abajo del loop, logré leer.");
    }
    Ok(())
}

fn process_publish(
    fixed_header: &FixedHeader,
    stream: &Arc<Mutex<TcpStream>>,
    fixed_header_bytes: &[u8; 2],
) -> Result<PublishMessage, Error> {
    println!("Recibo mensaje tipo Publish");
    let msg_bytes =
        get_message_decoded_in_bytes_from_stream(fixed_header, stream, fixed_header_bytes)?;
    let msg = PublishMessage::from_bytes(msg_bytes)?;
    println!("   Mensaje publish completo recibido: {:?}", msg);
    Ok(msg)
}

fn send_puback(msg: &PublishMessage, stream: &Arc<Mutex<TcpStream>>) -> Result<(), Error> {
    let option_packet_id = msg.get_packet_identifier();
    let packet_id = option_packet_id.unwrap_or(0);

    let ack = PubAckMessage::new(packet_id, 0);
    let ack_msg_bytes = ack.to_bytes();
    write_to_the_client(&ack_msg_bytes, stream)?;
    println!("   tipo publish: Enviado el ack: {:?}", ack);
    Ok(())
}

fn distribute_to_subscribers(
    msg: &PublishMessage,
    subs_by_topic: &ShHashmapType,
) -> Result<(), Error> {
    let topic = msg.get_topic();
    let msg_bytes = msg.to_bytes();
    if let Ok(subs_by_top) = subs_by_topic.lock() {
        if let Some(topic_subscribers) = subs_by_top.get(&topic) {
            println!(
                "   Se encontraron {} suscriptores al topic {:?}",
                topic_subscribers.len(),
                topic
            );
            //println!("Debug 1, pre for");
            for subscriber in topic_subscribers {
                write_to_the_client(&msg_bytes, subscriber)?;
                println!("      enviado mensaje publish a subscriber");
            }
            //println!("Debug 2, afuera del for");
        }
    }
    Ok(())
}

fn process_subscribe(
    fixed_header: &FixedHeader,
    stream: &Arc<Mutex<TcpStream>>,
    fixed_header_bytes: &[u8; 2],
) -> Result<SubscribeMessage, Error> {
    println!("Recibo mensaje tipo Subscribe");
    let msg_bytes =
        get_message_decoded_in_bytes_from_stream(fixed_header, stream, fixed_header_bytes)?;
    let msg = SubscribeMessage::from_bytes(msg_bytes)?;
    Ok(msg)
}

fn add_subscribers_to_topic(
    msg: &SubscribeMessage,
    stream: &Arc<Mutex<TcpStream>>,
    subs_by_topic: &ShHashmapType,
) -> Result<Vec<SubscribeReturnCode>, Error> {
    let mut return_codes = vec![];

    for (topic, _qos) in msg.get_topic_filters() {
        return_codes.push(SubscribeReturnCode::QoS1);
        let topic_s = topic.to_string();

        // Guarda una referencia (arc clone) al stream, en el vector de suscriptores al topic en cuestión
        if let Ok(mut subs_b_t) = subs_by_topic.lock() {
            subs_b_t
                .entry(topic_s)
                .or_insert_with(Vec::new)
                .push(stream.clone());
        }
        println!("   Se agregó el suscriptor al topic {:?}", topic);
    }
    Ok(return_codes)
}

fn send_suback(
    return_codes: Vec<SubscribeReturnCode>,
    stream: &Arc<Mutex<TcpStream>>,
) -> Result<(), Error> {
    let ack = SubAckMessage::new(0, return_codes);
    let msg_bytes = ack.to_bytes();
    write_to_the_client(&msg_bytes, stream)?;
    println!("   tipo subscribe: Enviado el ack: {:?}", ack);
    Ok(())
}

/// Escribe el mensaje en bytes `msg_bytes` por el stream hacia el cliente.
/// Puede devolver error si falla la escritura o el flush.
fn write_to_the_client(msg_bytes: &[u8], stream: &Arc<Mutex<TcpStream>>) -> Result<(), Error> {
    //println!("Debug 1.5, adentro de write");
    if let Ok(mut s) = stream.lock() {
        let _ = s.write(msg_bytes)?;
        s.flush()?;
    }
    Ok(())
}

/// Se ejecuta una vez recibido un `ConnectMessage` exitoso y devuelto un `ConnAckMessage` acorde.
/// Se puede empezar a recibir mensajes de otros tipos (`Publish`, `Subscribe`), de este cliente.
/// Recibe el `stream` para la comunicación con el cliente en cuestión.
/// Lee un mensaje.
fn continue_with_conection(
    stream: &Arc<Mutex<TcpStream>>,
    subs_by_topic: &ShHashmapType,
    fixed_header_info: &([u8; 2], FixedHeader),
) -> Result<(), Error> {
    let (fixed_header_bytes, fixed_header) = fixed_header_info;

    // Ahora sí ya puede haber diferentes tipos de mensaje.
    match fixed_header.get_message_type() {
        3 => {
            let msg = process_publish(fixed_header, stream, &fixed_header_bytes)?;

            send_puback(&msg, stream)?;

            distribute_to_subscribers(&msg, subs_by_topic)?;
        }
        8 => {
            let msg = process_subscribe(fixed_header, stream, &fixed_header_bytes)?;

            let return_codes = add_subscribers_to_topic(&msg, stream, subs_by_topic)?;

            send_suback(return_codes, stream)?;
        }
        _ => println!(
            "   ERROR: tipo desconocido: recibido: \n   {:?}",
            fixed_header
        ),
    };

    Ok(())
}

/// Procesa los mensajes entrantes de un dado cliente.
fn handle_client(
    stream: &Arc<Mutex<TcpStream>>,
    subs_by_topic: &ShHashmapType,
) -> Result<(), Error> {
    let (fixed_header_buf, fixed_header) = get_fixed_header_from_stream(stream)?;

    // El único tipo válido es el de connect, xq siempre se debe iniciar la comunicación con un connect.
    match fixed_header.get_message_type() {
        1 => {
            process_connect(&fixed_header, stream, &fixed_header_buf, subs_by_topic)?;
        }
        _ => {
            println!("Error, el primer mensaje recibido DEBE ser un connect.");
            println!("   recibido: {:?}", fixed_header);
            // ToDo: Leer de la doc qué hacer en este caso, o si solo ignoramos. []
        }
    };
    Ok(())
}

/// Lee el puerto por la consola, y devuelve la dirección IP y el puerto.
fn load_port() -> Result<(String, u16), Error> {
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

fn create_server(ip: String, port: u16) -> Result<TcpListener, Error> {
    let listener =
        TcpListener::bind(format!("{}:{}", ip, port)).expect("Error al enlazar el puerto");
    Ok(listener)
}

fn handle_incoming_connections(
    listener: TcpListener,
    subs_by_topic: ShHashmapType,
) -> Result<(), Error> {
    println!("Servidor iniciado. Esperando conexiones.");
    let mut handles = vec![];

    for stream_client in listener.incoming() {
        match stream_client {
            Ok(stream_client) => {
                let subs_by_topic_clone: ShHashmapType = subs_by_topic.clone();
                let handle = std::thread::spawn(move || {
                    let stream_client = Arc::new(Mutex::new(stream_client));
                    let _ = handle_client(&stream_client, &subs_by_topic_clone);
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

    Ok(())
}

fn main() -> Result<(), Error> {
    env_logger::init();

    let (ip, port) = load_port()?;

    let listener = create_server(ip, port)?;

    // Creo estructura subs_by_topic a usar (es un "Hashmap<topic, vec de subscribers>")
    // No es único hilo! al subscribe y al publish en cuestión lo hacen dos clientes diferentes! :)
    let subs_by_topic: ShHashmapType = Arc::new(Mutex::new(HashMap::new()));

    handle_incoming_connections(listener, subs_by_topic)?;

    Ok(())
}
