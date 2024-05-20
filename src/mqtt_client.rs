use crate::connect_message::ConnectMessage;
use crate::mqtt_client::io::ErrorKind;
use crate::mqtt_server_client_utils::{
    continuar_leyendo_bytes_del_msg, leer_fixed_header_de_stream_y_obt_tipo,
};
use crate::publish_flags::PublishFlags;
use crate::publish_message::PublishMessage;
use crate::subscribe_message::SubscribeMessage;
use std::io::{self, Error, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;
// Este archivo es nuestra librería MQTT para que use cada cliente que desee usar el protocolo.
use crate::connack_message::ConnackPacket;
use crate::fixed_header::FixedHeader;
use crate::puback_message::PubAckMessage;
use crate::suback_message::SubAckMessage;

#[allow(dead_code)]
/// MQTTClient es instanciado por cada cliente que desee utilizar el protocolo.
/// Posee el `stream` que usará para comunicarse con el `MQTTServer`.
/// El `stream` es un detalle de implementación que las apps que usen esta librería desconocen.
pub struct MQTTClient {
    stream: Arc<Mutex<TcpStream>>,
    handle_hijo: Option<JoinHandle<Result<(), Error>>>,
    rx: Receiver<Vec<u8>>,
}

impl MQTTClient {
    pub fn connect_to_broker(addr: &SocketAddr) -> Result<Self, Error> {
        //io::Result<()> {
        // Inicializaciones
        // Intenta conectar al servidor MQTT
        let stream_tcp = TcpStream::connect(addr)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;

        let stream = Arc::new(Mutex::new(stream_tcp));

        // Crea el mensaje tipo Connect y lo pasa a bytes
        let mut connect_msg = ConnectMessage::new(
            "rust-client",
            None, // will_topic
            None, // will_message
            Some("sistema-monitoreo"),
            Some("rustx123"),
        );
        let msg_bytes = connect_msg.to_bytes();

        // Intenta enviar el mensaje CONNECT al servidor MQTT
        {
            let mut s = stream.lock().unwrap();
            s.write(&msg_bytes)
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;
            s.flush()?;
        }
        println!("Envía connect: \n   {:?}", &connect_msg);

        // Más inicializaciones
        let (tx, rx) = mpsc::channel::<Vec<u8>>(); // [] que mande bytes, xq no hicimos un trait Message
        let mut stream_para_hijo = stream.clone();
        // Crea un hilo para leer desde servidor, y lo guarda para esperarlo
        let h = thread::spawn(move || {
            /*loop {
                // no hace nada, probando
            }*/
            thread::sleep(Duration::from_secs(3)); // [] aux, probando
            leer_desde_server(&mut stream_para_hijo, &tx) // [] Ahora que cambió desde afuera, pensar si stream es atributo o pasado
        });
        let mqtt = MQTTClient {
            stream,
            handle_hijo: Some(h),
            rx
        };
        // Fin inicializaciones.

        /*// Intenta leer la respuesta del servidor (CONNACK)
        let mut connack_response = [0; 4];
        {
            let mut s = mqtt.stream.lock().unwrap();
            s.read_exact(&mut connack_response)
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;
        }

        println!("Respuesta del servidor: \n   {:?}", &connack_response);*/

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
            }
            Err(e) => return Err(Error::new(ErrorKind::Other, e)),
        };
        let bytes_msg = pub_msg.to_bytes();
        // Lo envío
        {
            let mut s = self.stream.lock().unwrap();
            let _ = s.write(&bytes_msg)?;
            s.flush()?;
        }
        println!("Mqtt publish: envío bytes publish: \n   {:?}", bytes_msg);

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
    pub fn mqtt_subscribe(
        &mut self,
        packet_id: u16,
        topics_to_subscribe: Vec<(String, u8)>,
    ) -> Result<(), Error> {
        println!("-----------------");
        // Construyo subscribe
        let subscribe_msg = SubscribeMessage::new(packet_id, topics_to_subscribe);
        let subs_bytes = subscribe_msg.to_bytes();
        println!("Mqtt subscribe: enviando mensaje: \n   {:?}", subscribe_msg);
        // Lo envío
        thread::sleep(Duration::from_secs(3)); // [] aux, probando MIÁ QUÉ INTERESANTE
        {
            let mut s = self.stream.lock().unwrap();
            let _ = s.write(&subs_bytes)?;
            s.flush()?;
        }
        println!(
            "Mqtt subscribe: enviado mensaje en bytes: \n   {:?}",
            subs_bytes
        );

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

    /*/// Da una referencia adicional al `Arc<Mutex<TcpStream>>`.
    // (esta función no debería existir / usarse, dijimos que el stream era detalle de implementación desconocido desde afuera)
    pub fn get_stream(&self) -> Arc<Mutex<TcpStream>> {
        self.stream.clone()
    }*/

    /// Devuelve un elemento leído, para que le llegue a cada cliente que use esta librería.
    pub fn mqtt_receive_msg_from_subs_topic(&self) -> Result<Vec<u8>, mpsc::RecvError>  {
        self.rx.recv()  
    }

    /// Función que debe ser llamada por cada cliente que utilice la librería,
    /// como último paso, al finalizar.
    pub fn finalizar(&mut self) {
        if let Some(h) = self.handle_hijo.take() {
            let res = h.join();
            if res.is_err() {
                println!("Mqtt cliente: error al esperar hijo de lectura.");
            }
        }
    }
    
}

/// Función que ejecutará un hilo de MQTTClient, dedicado exclusivamente a la lectura.
fn leer_desde_server(stream: &mut Arc<Mutex<TcpStream>>, tx: &Sender<Vec<u8>>) -> Result<(), Error> {
    // Este bloque de código de acá abajo es similar a lo que hay en server,
    // pero la función que lee Un mensaje loo procesa de manera diferente.
    let mut fixed_header_buf = leer_fixed_header_de_stream_y_obt_tipo(&mut stream.clone())?;
    let ceros: &[u8; 2] = &[0; 2];
    let mut vacio = &fixed_header_buf == ceros;
    while !vacio {
        println!("Mqtt cliente leyendo: siguiente msj");
        let msg_bytes = leer_un_mensaje(&mut *stream, fixed_header_buf, tx)?; // esta función lee UN mensaje.
        /*match tx.send(&msg_bytes) {
            Ok(_) => println!("Mqtt cliente leyendo: se envía por tx exitosamente."),
            Err(_) => println!("Mqtt cliente leyendo: error al enviar por tx."),
        };*/
                                                          // Leo para la siguiente iteración
        fixed_header_buf = leer_fixed_header_de_stream_y_obt_tipo(&mut stream.clone())?;
        vacio = &fixed_header_buf == ceros;
    }
    Ok(())
}

/// Función interna que lee un mensaje, analiza su tipo, y lo procesa acorde a él.
fn leer_un_mensaje(
    stream: &mut Arc<Mutex<TcpStream>>,
    fixed_header_bytes: [u8; 2],
    tx: &Sender<Vec<u8>>
) -> Result<(), Error> {
    // He leído bytes de un fixed_header, tengo que ver de qué tipo es.
    let fixed_header = FixedHeader::from_bytes(fixed_header_bytes.to_vec());
    let tipo = fixed_header.get_tipo();
    println!("--------------------------");
    println!(
        "Mqtt cliente leyendo: Recibo fixed header, tipo: {}, bytes de fixed header leidos: {:?}",
        tipo, fixed_header
    );
    let msg_bytes: Vec<u8>;
    match tipo {
        2 => {
            // ConnAck
            println!("Mqtt cliente leyendo: recibo conn ack");
            msg_bytes =
                continuar_leyendo_bytes_del_msg(fixed_header, &mut *stream, &fixed_header_bytes)?;
            // Entonces tengo el mensaje completo
            let msg = ConnackPacket::from_bytes(&msg_bytes)?; //
            println!("   Mensaje conn ack completo recibido: {:?}", msg);      
            
            match tx.send(msg.to_bytes()) {
                Ok(_) => println!("Mqtt cliente leyendo: se envía por tx exitosamente."),
                Err(_) => println!("Mqtt cliente leyendo: error al enviar por tx."),
            };
        }
        3 => {
            println!("Mqtt cliente leyendo: recibo mensaje tipo Publish");
            // Esto ocurre cuando me suscribí a un topic, y server me envía los msjs del topic al que me suscribí
            msg_bytes =
                continuar_leyendo_bytes_del_msg(fixed_header, &mut *stream, &fixed_header_bytes)?;
            // Entonces tengo el mensaje completo
            let msg = PublishMessage::from_bytes(msg_bytes)?;
            println!("   Mensaje publish completo recibido: {:?}", msg);

            // Ahora ¿tengo que mandarle un PubAck? [] ver, imagino que sí
            if let Some(_packet_id) = msg.get_packet_identifier() {
                // Con el packet_id, marco en algún lado que recibí el ack.
            }
            //return Ok(msg.to_bytes());
            match tx.send(msg.to_bytes()) {
                Ok(_) => println!("Mqtt cliente leyendo: se envía por tx exitosamente."),
                Err(_) => println!("Mqtt cliente leyendo: error al enviar por tx."),
            };
        }
        4 => {
            // PubAck
            println!("Mqtt cliente leyendo: recibo pub ack");
            msg_bytes =
                continuar_leyendo_bytes_del_msg(fixed_header, &mut *stream, &fixed_header_bytes)?;
            // Entonces tengo el mensaje completo
            let msg = PubAckMessage::msg_from_bytes(msg_bytes)?; // []
            println!("   Mensaje pub ack completo recibido: {:?}", msg);
            //return Ok(msg.to_bytes());

            match tx.send(msg.to_bytes()) {
                Ok(_) => println!("Mqtt cliente leyendo: se envía por tx exitosamente."),
                Err(_) => println!("Mqtt cliente leyendo: error al enviar por tx."),
            };
        }
        9 => {
            // SubAck
            println!("Mqtt cliente leyendo: recibo sub ack");
            msg_bytes =
                continuar_leyendo_bytes_del_msg(fixed_header, &mut *stream, &fixed_header_bytes)?;
            // Entonces tengo el mensaje completo
            let msg = SubAckMessage::from_bytes(msg_bytes)?;
            println!("   Mensaje sub ack completo recibido: {:?}", msg);
            //return Ok(msg.to_bytes());

            match tx.send(msg.to_bytes()) {
                Ok(_) => println!("Mqtt cliente leyendo: se envía por tx exitosamente."),
                Err(_) => println!("Mqtt cliente leyendo: error al enviar por tx."),
            };
        }

        _ => {println!(
            "   ERROR: tipo desconocido: recibido: \n   {:?}",
            fixed_header);
            return Err(Error::new(ErrorKind::Other, "Tipo desconocido."));

        },
    };

    /*match tx.send(msg_bytes) {
        Ok(_) => println!("Mqtt cliente leyendo: se envía por tx exitosamente."),
        Err(_) => println!("Mqtt cliente leyendo: error al enviar por tx."),
    };*/
    
    Ok(())
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
