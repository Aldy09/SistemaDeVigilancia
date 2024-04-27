use log::{info, warn, error};
use std::net::{TcpStream, SocketAddr};
use std::io::{Read, Write};

fn main() {
    env_logger::init();

    info!("Este es un mensaje de información.");
    warn!("Esto es una advertencia.");
    error!("Esto es un error crítico.");

    let broker_addr = "127.0.0.1:9090".parse().expect("Dirección no válida");
    connect_to_broker(&broker_addr, "rust-client", Some("sistema-monitoreo"), Some("rustx123"));

}

fn connect_to_broker(addr: &SocketAddr, client_id: &str, username: Option<&str>, password: Option<&str>) {
    // Conecta al servidor MQTT
    let mut stream = TcpStream::connect(addr).expect("Error de conexión");

    // Construye el mensaje CONNECT
    let mut connect_msg: Vec<u8> = Vec::new();
    connect_msg.push(0x10); // Tipo de mensaje CONNECT
    connect_msg.push(0x00); // Longitud del mensaje CONNECT (se actualiza luego)
    connect_msg.extend_from_slice("MQTT".as_bytes()); // Protocolo "MQTT"
    connect_msg.push(0x04); // Versión 4 del protocolo MQTT

    // Flags de CONNECT: Clean Session, Username Flag y Password Flag
    let mut connect_flags: u8 = 0x00;
    if let Some(username) = username {
        connect_flags |= 0x80; // Bit 7: Username Flag
        connect_msg.extend_from_slice(username.as_bytes());
    }
    if let Some(password) = password {
        connect_flags |= 0x40; // Bit 6: Password Flag
        connect_msg.extend_from_slice(password.as_bytes());
    }
    connect_msg.push(connect_flags);

    connect_msg.push(0x00); // Keep Alive MSB
    connect_msg.push(0x0A); // Keep Alive LSB
    connect_msg.extend_from_slice(client_id.as_bytes()); // Client ID

    // Actualiza la longitud del mensaje CONNECT
    let msg_len = connect_msg.len() - 2;
    connect_msg[1] = msg_len as u8;

    // Envía el mensaje CONNECT al servidor MQTT
    stream.write_all(&connect_msg).expect("Error al enviar CONNECT");

    // Lee la respuesta del servidor (CONNACK)
    let mut connack_response = [0; 4];
    stream.read_exact(&mut connack_response).expect("Error al recibir respuesta");
    println!("Respuesta del servidor: {:?}", &connack_response);
}

