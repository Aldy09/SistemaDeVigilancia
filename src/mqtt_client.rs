use crate::connect_message::ConnectMessage;
use crate::mqtt_client::io::ErrorKind;
//use crate::puback_message::PubAckMessage;
use crate::publish_flags::PublishFlags;
use crate::publish_message::PublishMessage;
//use crate::suback_message::SubAckMessage;
use crate::subscribe_message::SubscribeMessage;
use std::io::{self, Error, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
// Este archivo es nuestra librería MQTT para que use cada cliente que desee usar el protocolo.

#[allow(dead_code)]
/// MQTTClient es instanciado por cada cliente que desee utilizar el protocolo.
/// Posee el `stream` que usará para comunicarse con el `MQTTServer`.
/// El `stream` es un detalle de implementación que las apps que usen esta librería desconocen.
pub struct MQTTClient {
    //stream: Option<TcpStream>,
    stream: Arc<Mutex<TcpStream>>,
}

impl MQTTClient {
    /*
    pub fn new(addr: &SocketAddr) -> io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        Ok(MQTTClient {
            stream,
        })
    }
    */

    pub fn connect_to_broker(
        addr: &SocketAddr,
        connect_msg: &mut ConnectMessage,
    ) -> Result<Self, Error> {
        //io::Result<()> {
        // Intenta conectar al servidor MQTT
        let stream = TcpStream::connect(addr)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;

        let mqtt = MQTTClient { stream: Arc::new(Mutex::new(stream)) };
        // Intenta enviar el mensaje CONNECT al servidor MQTT
        let msg_bytes = connect_msg.to_bytes();
        {
            let mut s = mqtt.stream.lock().unwrap();
            s.write(&msg_bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;
            s.flush()?;
        }
        println!("Envía connect: \n   {:?}", &connect_msg);

        // Intenta leer la respuesta del servidor (CONNACK)
        let mut connack_response = [0; 4];
        {
            let mut s = mqtt.stream.lock().unwrap();
            s.read_exact(&mut connack_response)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;
        }

        println!("Respuesta del servidor: \n   {:?}", &connack_response);

        Ok(mqtt)
    }

    // Nuestras apps clientes llamarán a esta función (los drones, etc)
    /// Función parte de la interfaz para uso de clientes del protocolo MQTT.
    /// Recibe el payload a enviar, y el topic al cual enviarlo.
    /// Devuelve Ok si el publish fue exitoso, es decir si se pudo enviar el mensaje Publish
    /// y se recibió un ack correcto. Devuelve error en caso contrario.
    pub fn mqtt_publish(&mut self, topic: &str, payload: &[u8]) -> Result<(), Error> {
        println!("-----------------");
        // Creo un msj publish
        let flags = PublishFlags::new(0, 1, 0)?;
        let result = PublishMessage::new(3, flags, topic, Some(1), payload);
        let pub_msg = match result {
            Ok(msg) => {
                println!("Mqtt publish: envío publish: \n   {:?}", msg);
                msg
            },
            Err(e) => return Err(Error::new(ErrorKind::Other, e)),
        };
        let bytes_msg = pub_msg.to_bytes();
        // Lo envío
        {
            let mut s = self.stream.lock().unwrap();
            s.write(&bytes_msg)?;
            s.flush()?;
        }

        /*// Leo la respuesta [] <-- no debe ir acá, ver comentario abajo.
        let mut bytes_rta_leida = [0; 5];
        {
            let mut s = self.stream.lock().unwrap();
            let _cant_leida = s.read(&mut bytes_rta_leida)?;
        }

        let puback_msg = PubAckMessage::msg_from_bytes(bytes_rta_leida.to_vec())?; // []
        println!("Mqtt publish: recibo este pub ack: \n   {:?}", puback_msg);*/

        Ok(())
    }

    // Nuestras apps clientes llamarán a esta función (los drones, etc)
    /// Función parte de la interfaz para uso de clientes del protocolo MQTT.
    /// Recibe el packet id, y un vector de topics a los cuales cliente desea suscribirse.
    pub fn mqtt_subscribe(&mut self, packet_id: u16, topics_to_subscribe: Vec<(String, u8)>) -> Result<(), Error> {
        println!("-----------------");
        // Construyo subscribe
        let subscribe_msg = SubscribeMessage::new(packet_id, topics_to_subscribe);
        let subs_bytes = subscribe_msg.to_bytes();
        println!("Mqtt subscribe: enviando mensaje: \n   {:?}", subscribe_msg);
        // Lo envío
        {
            let mut s = self.stream.lock().unwrap();
            s.write(&subs_bytes)?;
            s.flush()?;
        }
        println!("Mqtt subscribe: enviado mensaje en bytes: \n   {:?}", subs_bytes);

        /*// Leo la respuesta
        let mut bytes_rta_leida = [0; 6]; // [] Aux temp: 6 para 1 elem, 8 p 2, 10 p 3, en realidad hay que leer el fixed hdr como en server.
        {
            let mut s = self.stream.lock().unwrap();
            let cant_leida = s.read(&mut bytes_rta_leida)?;
            println!("Mqtt subscribe: cant_leida de bytes de sub ack: {}", cant_leida);
            //println!("Mqtt subscribe: bytes de sub ack: {:?}", bytes_rta_leida);
        }

        let ack = SubAckMessage::from_bytes(bytes_rta_leida.to_vec())?; // [] <-- este paso rompe, da error, si no es el tipo esperado, pero por el stream podría venir el ack de un publish por ejemplo.
        println!("Mqtt subscribe: recibo ack: \n   {:?}", ack); // y si ya hice el read y no me interesaba, lo perdí. Debe haber un hilo para los reads de ack, y otro para enviar. */

        Ok(())
    }

    /// Función que devuelve un struct MQTTClient que contiene una referencia adicional
    /// del `Arc<Mutex<TcpStream>>`.
    pub fn clone(&self) -> Self {
        MQTTClient { stream: self.stream.clone() }
    }

    /// Da una referencia adicional al `Arc<Mutex<TcpStream>>`.
    pub fn get_stream(&self) -> Arc<Mutex<TcpStream>> {
        self.stream.clone()
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
}
*/
