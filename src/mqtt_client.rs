use crate::connect_message::ConnectMessage;
use crate::publish_flags::PublishFlags;
use crate::publish_message::PublishMessage;
use crate::puback_message::PubAckMessage;
use std::io::{self, Read, Write, Error};
use std::net::{SocketAddr, TcpStream};
// Este archivo es nuestra librería MQTT para que use cada cliente que desee usar el protocolo.

#[allow(dead_code)]
/// MQTTClient es instanciado por cada cliente que desee utilizar el protocolo.
/// Posee el `stream` que usará para comunicarse con el `MQTTServer`.
/// El `stream` es un detalle de implementación que las apps que usen esta librería desconocen.
pub struct MQTTClient {
    //stream: Option<TcpStream>,
    stream: TcpStream,
}

impl MQTTClient {
    pub fn connect_to_broker(addr: &SocketAddr, connect_msg: &mut ConnectMessage) -> Result<Self, Error>{//io::Result<()> {
        // Intenta conectar al servidor MQTT
        let mut stream = TcpStream::connect(addr)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;

        // Intenta enviar el mensaje CONNECT al servidor MQTT
        let msg_bytes = connect_msg.to_bytes();
        stream
            .write_all(&msg_bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;

        // Intenta leer la respuesta del servidor (CONNACK)
        let mut connack_response = [0; 4];
        stream
            .read_exact(&mut connack_response)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;

        println!("Respuesta del servidor: {:?}", &connack_response);

        Ok(MQTTClient { stream })
    }

    // Nuestras apps clientes llamarán a esta función (los drones, etc)
    /// Función parte de la interfaz para uso de clientes del protocolo MQTT.
    /// Recibe el payload a enviar, y el topic al cual enviarlo.
    pub fn mqtt_publish(&mut self, topic: &str, payload: &[u8]) -> Result<(), Error> {
        println!("-----------------");
        // Construyo publish
        // Creo un pub msg
        let flags = PublishFlags::new(0,0,0)?;
        let string = String::from(topic);
        let pub_msg = PublishMessage::new(flags, string, 1, payload); //"hola".as_bytes() );

        let bytes_msg = pub_msg.to_bytes();
        //if let Some(mut s) = self.stream {
        //s.write_all(&bytes_msg)?;
        println!("   Mensaje publish en bytes a enviar: {:?}", bytes_msg);
        self.stream.write_all(&bytes_msg)?;

        //let max_buff_size: usize = PubAckMessage::max_posible_msg_size();
        let mut bytes_rta_leida = [0; 5];
        let cant_leida = self.stream.read(&mut bytes_rta_leida);
        match cant_leida {
            Ok(u) => {
                println!("READ correcto en mqtt_publish, usize: {}", u);
            },
            Err(e) => {
                println!("ERROR al leer: {:?}", e);
                println!("Lo leído hasta dar error, fue: {:?}", bytes_rta_leida);
            },
        }
        println!("-----------------");

        let puback_msg = PubAckMessage::msg_from_bytes(bytes_rta_leida.to_vec())?; // []
        println!("RECIBO ESTE PUB ACK MSG: {:?}", puback_msg);
        //}

        
        let _msg_reconstruido = PublishMessage::pub_msg_from_bytes(bytes_msg);
        println!("-----------------");
        Ok(())
    }


}

/*
#[cfg(test)]
mod test {
    use super::MQTTClient;

    #[test]
    fn test_1_publish_probando(){
        let mqtt_client = MQTTClient::new();

        // Yo quiero probar solamente esta función y no el connect ahora; ver cómo.
        let res = mqtt_client.mqtt_publish("topic3", "hola mundo");
        //assert!(res.is_ok()); // Va a fallar, xq no llamé a connect primero,
                                // y xq hay stream socket involucrado (y no está levantado el server, es un unit test).
                                // ToDo: hay que ver cómo testear esto con un archivo y mocks.
    }
}*/