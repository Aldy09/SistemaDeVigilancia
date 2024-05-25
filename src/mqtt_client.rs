use crate::connect_message::ConnectMessage;
use crate::mqtt_client::io::ErrorKind;
use crate::mqtt_server_client_utils::{
    get_fixed_header_from_stream, get_whole_message_in_bytes_from_stream, write_to_the_client,
};
use crate::publish_flags::PublishFlags;
use crate::publish_message::PublishMessage;
use crate::subscribe_message::SubscribeMessage;
use std::io::{self, Error};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
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
    rx: Receiver<PublishMessage>,
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

        // Intenta enviar el mensaje CONNECT al servidor MQTT
        let msg_bytes = connect_msg.to_bytes();
        write_to_the_client(&msg_bytes, &stream)?;
        println!("Envía connect: \n   {:?}", &connect_msg);
        println!("   El Connect en bytes: {:?}", msg_bytes);

        // Más inicializaciones
        let (tx, rx) = mpsc::channel::<PublishMessage>(); // [] que mande bytes, xq no hicimos un trait Message
        let mut stream_para_hijo = stream.clone();
        // Crea un hilo para leer desde servidor, y lo guarda para esperarlo
        let h = thread::spawn(move || {
            leer_desde_server(&mut stream_para_hijo, &tx) // [] Ahora que cambió desde afuera, pensar si stream es atributo o pasado
        });
        let mqtt = MQTTClient {
            stream,
            handle_hijo: Some(h),
            rx,
        };
        // Fin inicializaciones.

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
        let flags = PublishFlags::new(0, 2, 0)?;
        let result = PublishMessage::new(3, flags, topic, Some(1), payload);
        let pub_msg = match result {
            Ok(msg) => {
                println!("Mqtt publish: envío publish: \n   {:?}", msg);
                msg
            }
            Err(e) => return Err(Error::new(ErrorKind::Other, e)),
        };

        // Lo envío
        let bytes_msg = pub_msg.to_bytes();
        write_to_the_client(&bytes_msg, &self.stream)?;
        println!("Mqtt publish: envío bytes publish: \n   {:?}", bytes_msg);

        Ok(())
    }

    // Nuestras apps clientes llamarán a esta función (los drones, etc)
    /// Función parte de la interfaz para uso de clientes del protocolo MQTT.
    /// Recibe el packet id, y un vector de topics a los cuales cliente desea suscribirse.
    pub fn mqtt_subscribe(
        &mut self,
        packet_id: u16,
        topics_to_subscribe: Vec<String>,
    ) -> Result<(), Error> {
        println!("-----------------");
        // Construyo subscribe
        let subscribe_msg = SubscribeMessage::new(packet_id, topics_to_subscribe);
        println!("Mqtt subscribe: enviando mensaje: \n   {:?}", subscribe_msg);

        // Lo envío
        let subs_bytes = subscribe_msg.to_bytes();
        write_to_the_client(&subs_bytes, &self.stream)?;
        println!(
            "Mqtt subscribe: enviado mensaje en bytes: \n   {:?}",
            subs_bytes
        );

        Ok(())
    }

    /// Devuelve un elemento leído, para que le llegue a cada cliente que use esta librería.
    pub fn mqtt_receive_msg_from_subs_topic(&self) -> Result<PublishMessage, mpsc::RecvError> {
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
fn leer_desde_server(
    stream: &mut Arc<Mutex<TcpStream>>,
    tx: &Sender<PublishMessage>,
) -> Result<(), Error> {
    // Este bloque de código de acá abajo es similar a lo que hay en server,
    // pero la función que lee un mensaje lo procesa de manera diferente.
    let mut fixed_header_info = get_fixed_header_from_stream(&stream.clone())?; // [] acá estamos
    let ceros: &[u8; 2] = &[0; 2];
    let mut vacio = &fixed_header_info.0 == ceros;
    while !vacio {
        println!("Mqtt cliente leyendo: siguiente msj");
        leer_un_mensaje(stream, &fixed_header_info, tx)?; // esta función lee UN mensaje.

        // Leo fixed header para la siguiente iteración del while, como la función utiliza timeout, la englobo en un loop
        // cuando leyío algo, corto el loop y continúo a la siguiente iteración del while
        println!("Mqtt cliente leyendo: esperando más mensajes.");
        loop {
            if let Ok((fixed_h_buf, fixed_h)) = get_fixed_header_from_stream(&stream.clone()) {
                println!("While: leí bien.");
                // Guardo lo leído y comparo para siguiente vuelta del while
                fixed_header_info = (fixed_h_buf, fixed_h);
                vacio = &fixed_header_info.0 == ceros;
                break;
            };
            thread::sleep(Duration::from_millis(300)); // []
        }
    }
    Ok(())
}

/// Función interna que lee un mensaje, analiza su tipo, y lo procesa acorde a él.
fn leer_un_mensaje(
    stream: &Arc<Mutex<TcpStream>>,
    fixed_header_info: &([u8; 2], FixedHeader),
    tx: &Sender<PublishMessage>,
) -> Result<(), Error> {
    // He leído bytes de un fixed_header, tengo que ver de qué tipo es.
    let (fixed_header_bytes, fixed_header) = fixed_header_info;
    // Soy client, siempre inicio yo la conexión, puedo recibir distintos tipos de mensaje.
    let tipo = fixed_header.get_message_type();
    let msg_bytes: Vec<u8>;

    match tipo {
        2 => {
            // ConnAck
            println!("Mqtt cliente leyendo: recibo conn ack");
            msg_bytes = get_whole_message_in_bytes_from_stream(
                fixed_header,
                stream,
                fixed_header_bytes,
                "conn ack",
            )?;
            // Entonces tengo el mensaje completo
            let msg = ConnackPacket::from_bytes(&msg_bytes)?; //
            println!("   Mensaje conn ack completo recibido: {:?}", msg);
        }
        3 => {
            // Publish
            println!("Mqtt cliente leyendo: RECIBO MENSAJE TIPO PUBLISH");
            // Esto ocurre cuando me suscribí a un topic, y server me envía los msjs del topic al que me suscribí
            msg_bytes = get_whole_message_in_bytes_from_stream(
                fixed_header,
                stream,
                fixed_header_bytes,
                "publish",
            )?;
            // Entonces tengo el mensaje completo
            let msg = PublishMessage::from_bytes(msg_bytes)?;
            //println!("   Mensaje publish completo recibido: {:?}", msg);

            // Ahora ¿tengo que mandarle un PubAck? [] ver, imagino que sí
            if let Some(_packet_id) = msg.get_packet_identifier() {
                // Con el packet_id, marco en algún lado que recibí el ack.
            }

            match tx.send(msg) {
                Ok(_) => println!("Mqtt cliente leyendo: se envía por tx exitosamente."),
                Err(_) => println!("Mqtt cliente leyendo: error al enviar por tx."),
            };
        }
        4 => {
            // PubAck
            println!("Mqtt cliente leyendo: recibo pub ack");
            msg_bytes = get_whole_message_in_bytes_from_stream(
                fixed_header,
                stream,
                fixed_header_bytes,
                "pub ack",
            )?;
            // Entonces tengo el mensaje completo
            let msg = PubAckMessage::msg_from_bytes(msg_bytes)?; // []
            println!("   Mensaje pub ack completo recibido: {:?}", msg);
        }
        9 => {
            // SubAck
            println!("Mqtt cliente leyendo: recibo sub ack");
            msg_bytes = get_whole_message_in_bytes_from_stream(
                fixed_header,
                stream,
                fixed_header_bytes,
                "sub ack",
            )?;
            // Entonces tengo el mensaje completo
            let msg = SubAckMessage::from_bytes(msg_bytes)?;
            println!("   Mensaje sub ack completo recibido: {:?}", msg);
        }

        _ => {
            println!(
                "   ERROR: tipo desconocido: recibido: \n   {:?}",
                fixed_header
            );
            return Err(Error::new(ErrorKind::Other, "Tipo desconocido."));
        }
    };

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
