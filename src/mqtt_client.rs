use crate::messages::connect_message::ConnectMessage;
use crate::messages::disconnect_message::DisconnectMessage;
use crate::messages::publish_flags::PublishFlags;
use crate::messages::publish_message::PublishMessage;
use crate::messages::subscribe_message::SubscribeMessage;
use crate::mqtt_client::io::ErrorKind;
use crate::mqtt_server_client_utils::{
    get_fixed_header_from_stream, get_whole_message_in_bytes_from_stream, send_puback,
    write_message_to_stream,
};
use std::collections::HashMap;
use std::io::{self, Error};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
// Este archivo es nuestra librería MQTT para que use cada cliente que desee usar el protocolo.
use crate::fixed_header::FixedHeader;
use crate::messages::connack_message::ConnackMessage;
use crate::messages::puback_message::PubAckMessage;
use crate::messages::suback_message::SubAckMessage;

#[allow(dead_code)]
/// MQTTClient es instanciado por cada cliente que desee utilizar el protocolo.
/// Posee el `stream` que usará para comunicarse con el `MQTTServer`.
/// El `stream` es un detalle de implementación que las apps que usen esta librería desconocen.
pub struct MQTTClient {
    stream: Arc<Mutex<TcpStream>>,
    handle_hijo: Option<JoinHandle<()>>,
    rx: Option<Receiver<PublishMessage>>,
    available_packet_id: u16, // mantiene el primer packet_id disponible para ser utilizado
    //acks_by_packet_id: // read control messages:
    read_connack: Arc<Mutex<bool>>,

    read_acks: Arc<Mutex<HashMap<u16, bool>>>,
}

impl MQTTClient {
    pub fn mqtt_connect_to_broker(client_id: &str, addr: &SocketAddr) -> Result<Self, Error> {
        // Inicializaciones
        // Intenta conectar al servidor MQTT
        let stream_tcp = TcpStream::connect(addr)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;

        let stream = Arc::new(Mutex::new(stream_tcp));
    
        // Crea el mensaje tipo Connect y lo pasa a bytes
        let mut connect_msg = ConnectMessage::new(
            client_id,
            None, // will_topic
            None, // will_message
            Some("usuario0"),
            Some("rustx123"),
        );

        // Intenta enviar el mensaje CONNECT al servidor MQTT
        let msg_bytes = connect_msg.to_bytes();
        write_message_to_stream(&msg_bytes, &stream)?;
        println!("Envía connect: \n   {:?}", &connect_msg);
        println!("   El Connect en bytes: {:?}", msg_bytes);

        // Más inicializaciones
        let (mut mqtt, tx) = MQTTClient::mqtt_new(stream);
        //

        let self_p_hijo = mqtt.clone_refs_para_hijo_lectura();
        // Crea un hilo para leer desde servidor, y lo guarda para esperarlo
        let h: JoinHandle<()> = thread::spawn(move || {
            let _res = self_p_hijo.leer_desde_server(&tx); // []
        });
        mqtt.set_hijo_a_esperar(h);

        // Fin inicializaciones.

        // Espero que el hijo que lee reciba y me informe que recibió el ack.
        let mut llega_el_ack = false;
        while !llega_el_ack {
            if let Ok(llega_connack) = mqtt.read_connack.lock() {
                if *llega_connack {
                    // Llegó el ack
                    llega_el_ack = true;
                    println!("CONN: LLEGA EL ACK"); // debug []
                }
            }
        }

        Ok(mqtt)
    }

    fn mqtt_new(stream: Arc<Mutex<TcpStream>>) -> (Self, Sender<PublishMessage>) {
        let (tx, rx) = mpsc::channel::<PublishMessage>();

        let mqtt = MQTTClient {
            stream: stream.clone(),
            handle_hijo: None,
            rx: Some(rx),
            available_packet_id: 0,
            read_connack: Arc::new(Mutex::new(false)),
            read_acks: Arc::new(Mutex::new(HashMap::new())),
        };
        (mqtt, tx)
    }

    // Nuestras apps clientes llamarán a esta función (los drones, etc)
    /// Función parte de la interfaz para uso de clientes del protocolo MQTT.
    /// Recibe el payload a enviar, y el topic al cual enviarlo.
    /// Devuelve Ok si el publish fue exitoso, es decir si se pudo enviar el mensaje Publish
    /// y se recibió un ack correcto. Devuelve error en caso contrario.
    pub fn mqtt_publish(&mut self, topic: &str, payload: &[u8]) -> Result<PublishMessage, Error> {
        println!("-----------------");
        let packet_id = self.generate_packet_id();
        // Creo un msj publish
        let flags = PublishFlags::new(0, 1, 0)?;
        let result = PublishMessage::new(3, flags, topic, Some(packet_id), payload);
        let publish_message = match result {
            Ok(msg) => {
                println!("Mqtt publish: envío publish: \n   {:?}", msg);
                msg
            }
            Err(e) => return Err(Error::new(ErrorKind::Other, e)),
        };

        // Lo envío
        let bytes_msg = publish_message.to_bytes();
        write_message_to_stream(&bytes_msg, &self.stream)?;
        println!("Mqtt publish: envío bytes publish: \n   {:?}", bytes_msg);

        Ok(publish_message)
    }

    // Nuestras apps clientes llamarán a esta función (los drones, etc)
    /// Función parte de la interfaz para uso de clientes del protocolo MQTT.
    /// Recibe el packet id, y un vector de topics a los cuales cliente desea suscribirse.
    pub fn mqtt_subscribe(&mut self, topics_to_subscribe: Vec<String>) -> Result<SubscribeMessage, Error> {
        let packet_id = self.generate_packet_id();
        println!("-----------------");
        // Construyo subscribe
        let subscribe_msg = SubscribeMessage::new(packet_id, topics_to_subscribe);
        println!("Mqtt subscribe: enviando mensaje: \n   {:?}", subscribe_msg);

        // Lo envío
        let subs_bytes = subscribe_msg.to_bytes();
        write_message_to_stream(&subs_bytes, &self.stream)?;
        println!(
            "Mqtt subscribe: enviado mensaje en bytes: \n   {:?}",
            subs_bytes
        );

        Ok(subscribe_msg)
    }

    /// Devuelve un elemento leído, para que le llegue a cada cliente que use esta librería.
    pub fn mqtt_receive_msg_from_subs_topic(
        &self,
        //) -> Result<PublishMessage, mpsc::RecvTimeoutError> {
    ) -> Result<PublishMessage, Error> {
        // Veo si tengo el rx (hijo no lo tiene)
        if let Some(rx) = &self.rx {
            // Recibo un PublishMessage por el rx, para hacérselo llegar al cliente real que usa la librería
            // Leo
            match rx.recv_timeout(Duration::from_micros(300)) {
                Ok(msg) => Ok(msg),
                // (mapeo el error, por compatibilidad de tipos)
                Err(e) => match e {
                    mpsc::RecvTimeoutError::Timeout => Err(Error::new(ErrorKind::TimedOut, e)),
                    mpsc::RecvTimeoutError::Disconnected => {
                        Err(Error::new(ErrorKind::NotConnected, e))
                    }
                },
            }
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "Error: no está seteado el rx.",
            ))
        }
    }

    /// Envía mensaje disconnect, y cierra la conexión con el servidor.
    pub fn mqtt_disconnect(&self) -> Result<(), Error> {
        let msg = DisconnectMessage::new();

        // Lo envío
        let bytes_msg = msg.to_bytes();
        write_message_to_stream(&bytes_msg, &self.stream)?;
        println!("Mqtt disconnect: bytes {:?}", bytes_msg);

        // Cerramos la conexión con el servidor
        if let Ok(s) = self.stream.lock() {
            match s.shutdown(Shutdown::Both) {
                Ok(_) => println!("Mqtt disconnect: Conexión terminada con éxito"),
                Err(e) => println!("Mqtt disconnect: Error al terminar la conexión: {:?}", e),
            }
        }

        Ok(())
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

    /// Setea el handle del hijo para poder esperarlo y terminar correctamente.
    fn set_hijo_a_esperar(&mut self, h: JoinHandle<()>) {
        self.handle_hijo = Some(h);
    }

    /// Devuelve otro struct MQTTClient, con referencias a las mismas estructuras englobadas en ^Arc Mutex^
    /// que utiliza el MQTTClient para el cual se está llamando a esta función, con la diferencia de que
    /// los campos para esperar al hijo y para recibir mensajes publish están seteados en `None` ya que no son
    /// de interés para un hijo del MQTTClient original.
    fn clone_refs_para_hijo_lectura(&self) -> Self {
        Self {
            stream: self.stream.clone(),
            handle_hijo: None,
            rx: None,
            available_packet_id: self.available_packet_id,
            read_connack: self.read_connack.clone(),
            read_acks: self.read_acks.clone(),
        }
    }

    /// Función que ejecutará un hilo de MQTTClient, dedicado exclusivamente a la lectura.
    fn leer_desde_server(&self, tx: &Sender<PublishMessage>) -> Result<(), Error> {
        // Este bloque de código de acá abajo es similar a lo que hay en server,
        // pero la función que lee un mensaje lo procesa de manera diferente.

        // Inicio primer mensaje
        let mut fixed_header_info: ([u8; 2], FixedHeader);
        let ceros: &[u8; 2] = &[0; 2];
        let mut vacio: bool;

        //vacio = &fixed_header_info.0 == ceros;
        println!("Mqtt cliente leyendo: esperando más mensajes.");
        loop {
            if let Ok((fixed_h_buf, fixed_h)) = get_fixed_header_from_stream(&self.stream.clone()) {
                println!("While: leí bien.");
                // Guardo lo leído y comparo para siguiente vuelta del while
                fixed_header_info = (fixed_h_buf, fixed_h);
                vacio = &fixed_header_info.0 == ceros;
                break;
            };
            thread::sleep(Duration::from_millis(300)); // []
        }
        // Fin

        /*let mut fixed_header_info = get_fixed_header_from_stream(&stream.clone())?; // [] acá estamos
        let ceros: &[u8; 2] = &[0; 2];
        let mut vacio = &fixed_header_info.0 == ceros;*/
        while !vacio {
            println!("Mqtt cliente leyendo: siguiente msj");
            self.leer_un_mensaje(&fixed_header_info, tx)?; // esta función lee UN mensaje.

            // Leo fixed header para la siguiente iteración del while, como la función utiliza timeout, la englobo en un loop
            // cuando leyío algo, corto el loop y continúo a la siguiente iteración del while
            println!("Mqtt cliente leyendo: esperando más mensajes.");
            loop {
                if let Ok((fixed_h_buf, fixed_h)) =
                    get_fixed_header_from_stream(&self.stream.clone())
                {
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
        &self,
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
                    &self.stream,
                    fixed_header_bytes,
                    "conn ack",
                )?;
                // Entonces tengo el mensaje completo
                let msg = ConnackMessage::from_bytes(&msg_bytes)?; //
                println!("   Mensaje conn ack completo recibido: {:?}", msg);

                // Marco que el ack fue recibido, para que el otro hilo pueda enterarse
                if let Ok(mut read_connack_locked) = self.read_connack.lock() {
                    *read_connack_locked = true;
                    // [] acá
                }
            }
            3 => {
                // Publish
                println!("Mqtt cliente leyendo: RECIBO MENSAJE TIPO PUBLISH");
                // Esto ocurre cuando me suscribí a un topic, y server me envía los msjs del topic al que me suscribí
                msg_bytes = get_whole_message_in_bytes_from_stream(
                    fixed_header,
                    &self.stream,
                    fixed_header_bytes,
                    "publish",
                )?;
                // Entonces tengo el mensaje completo
                let msg = PublishMessage::from_bytes(msg_bytes)?;
                //println!("   Mensaje publish completo recibido: {:?}", msg);

                // Le respondo un pub ack
                let _ = send_puback(&msg, &self.stream);

                // Envío por el channel para que le llegue al cliente real (app cliente)
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
                    &self.stream,
                    fixed_header_bytes,
                    "pub ack",
                )?;
                // Entonces tengo el mensaje completo
                let msg = PubAckMessage::msg_from_bytes(msg_bytes)?;
                println!("   Mensaje pub ack completo recibido: {:?}", msg);
            }
            9 => {
                // SubAck
                println!("Mqtt cliente leyendo: recibo sub ack");
                msg_bytes = get_whole_message_in_bytes_from_stream(
                    fixed_header,
                    &self.stream,
                    fixed_header_bytes,
                    "sub ack",
                )?;
                // Entonces tengo el mensaje completo
                let msg = SubAckMessage::from_bytes(msg_bytes)?;
                println!("   Mensaje sub ack completo recibido: {:?}", msg);
            }

            14 => {
                // Disconnect
                println!("Mqtt cliente leyendo: recibo disconnect");

                // Cerramos la conexión con el servidor
                if let Ok(s) = self.stream.lock() {
                    match s.shutdown(Shutdown::Both) {
                        Ok(_) => println!("Conexión terminada con éxito"),
                        Err(e) => println!("Error al terminar la conexión: {:?}", e),
                    }
                }
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

    /// Devuelve el packet_id a usar para el siguiente mensaje enviado.
    /// Incrementa en 1 el atributo correspondiente, debido a la llamada anterior, y devuelve el valor a ser usado
    /// en el envío para el cual fue llamada esta función.
    fn generate_packet_id(&mut self) -> u16 {
        //self.last_used_packet_id += 1;
        //self.last_used_packet_id
        self.available_packet_id += 1;
        self.available_packet_id
    }
}
