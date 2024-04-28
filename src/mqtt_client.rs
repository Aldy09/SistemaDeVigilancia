use crate::connect_message::ConnectMessage;
use std::io::{Read, Write};
use std::net::{TcpStream, SocketAddr};



pub struct MQTTClient {}

impl MQTTClient {
    pub fn new(){
        
    }
    pub fn connect_to_broker(addr: &SocketAddr, connect_msg: &ConnectMessage) {
        // Conecta al servidor MQTT
        let mut stream = TcpStream::connect(addr).expect("Error de conexión");

        // Envía el mensaje CONNECT al servidor MQTT
        let msg_bytes = connect_msg.to_bytes();
        stream
            .write_all(&msg_bytes)
            .expect("Error al enviar CONNECT");

        // Lee la respuesta del servidor (CONNACK)
        let mut connack_response = [0; 4];
        stream
            .read_exact(&mut connack_response)
            .expect("Error al recibir respuesta");
        println!("Respuesta del servidor: {:?}", &connack_response);
    }
}
