use crate::connect_message::ConnectMessage;
use std::io::{self,Read, Write};
use std::net::{TcpStream, SocketAddr};


#[allow(dead_code)]
pub struct MQTTClient {


}

impl MQTTClient {
    pub fn new()->MQTTClient{
        MQTTClient{}
        
    }
    pub fn connect_to_broker(addr: &SocketAddr, connect_msg: &ConnectMessage) -> io::Result<()> {
        // Intenta conectar al servidor MQTT
        let mut stream = TcpStream::connect(addr).map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;
    
        // Intenta enviar el mensaje CONNECT al servidor MQTT
        let msg_bytes = connect_msg.to_bytes();
        stream.write_all(&msg_bytes).map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;
    
        // Intenta leer la respuesta del servidor (CONNACK)
        let mut connack_response = [0; 4];
        stream.read_exact(&mut connack_response).map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;
    
        println!("Respuesta del servidor: {:?}", &connack_response);
    
        Ok(())
    }
}
