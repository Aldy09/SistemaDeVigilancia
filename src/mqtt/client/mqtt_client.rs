use crate::mqtt::messages::connect_return_code::ConnectReturnCode;
//use crate::mqtt::messages::message::Message;
use crate::mqtt::messages::{
    connect_message::ConnectMessage, disconnect_message::DisconnectMessage,
    publish_flags::PublishFlags, publish_message::PublishMessage,
    subscribe_message::SubscribeMessage,
};
/*use crate::mqtt::mqtt_utils::{self, mqtt_server_client_utils::{
    get_fixed_header_from_stream, get_whole_message_in_bytes_from_stream, send_puback,
    write_message_to_stream, get_fixed_header_from_stream_without_timeout,
}};*/

use crate::mqtt::mqtt_utils::aux_server_utils::{get_fixed_header_from_stream, get_fixed_header_from_stream_for_conn, get_whole_message_in_bytes_from_stream, is_disconnect_msg, send_puback, shutdown, write_message_to_stream};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::io::{self, Error};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
// Este archivo es nuestra librería MQTT para que use cada cliente que desee usar el protocolo.
use crate::mqtt::messages::{
    connack_message::ConnackMessage, puback_message::PubAckMessage, suback_message::SubAckMessage,
};
use crate::mqtt::mqtt_utils::fixed_header::FixedHeader;

#[allow(dead_code)]
/// MQTTClient es instanciado por cada cliente que desee utilizar el protocolo.
/// Posee el `stream` que usará para comunicarse con el `MQTTServer`.
/// El `stream` es un detalle de implementación que las apps que usen esta librería desconocen.

/*#[derive(Debug)]
pub struct MQTTClient {
    stream: Arc<Mutex<TcpStream>>,
    handle_child: Option<JoinHandle<()>>,
    rx: Option<Receiver<PublishMessage>>,
    available_packet_id: u16, // mantiene el primer packet_id disponible para ser utilizado
    //acks_by_packet_id: // read control messages:
    read_connack: Arc<Mutex<bool>>,

    read_acks: Arc<Mutex<HashMap<u16, bool>>>,
}*/

//type StreamType = Arc<Mutex<TcpStream>>;
type StreamType = TcpStream;
//type GenericMessage = dyn Send;
#[derive(Debug)]
pub struct MQTTClient {
    stream: StreamType,
    children: Option<Vec<JoinHandle<()>>>,
    available_packet_id: u16, // mantiene el primer packet_id disponible para ser utilizado
    //acks_by_packet_id: // read control messages:
    read_acks: Arc<Mutex<HashMap<u16, bool>>>,
    //acks_tx: Sender<GenericMessage>,
}

impl MQTTClient {
    /// Función que crea e inicializa una instancia de `MQTTClient`, y realiza la conexión TCP con el servidor.
    /// Recibe el `client_id` del cliente que llama a esta función, y la `addr` que es la dirección del
    /// servidor al que conectarse.
    /// Devuelve el struct mencionado, o bien un error en caso de que fallara el intento de conexión.
    pub fn mqtt_connect_to_broker(client_id: &str, addr: &SocketAddr) -> Result<(Self, Receiver<PublishMessage>), Error> {
        // Inicializaciones
        // Intenta conectar al servidor MQTT
        let stream_tcp = TcpStream::connect(addr)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;

        //let mut stream = Arc::new(Mutex::new(stream_tcp));
        let mut stream = stream_tcp;

        // Crea el mensaje tipo Connect y lo pasa a bytes
        let mut connect_msg = ConnectMessage::new(
            client_id.to_string(),
            None, // will_topic
            None, // will_message
            Some("usuario0".to_string()),
            Some("rustx123".to_string()),
        );

        // Intenta enviar el mensaje CONNECT al servidor MQTT
        let msg_bytes = connect_msg.to_bytes();
        write_message_to_stream(&msg_bytes, &mut stream)?;
        println!("Envía connect: \n   {:?}", &connect_msg);

        println!("Mqtt cliente leyendo: esperando connack.");
        // Leo un fixed header, deberá ser de un connect
        read_connack(&mut stream)?;

        let (mqtt, rx) = Self::initialize(stream)?;

        Ok((mqtt, rx))
    }

    /// Realiza las inicializaciones necesarias para el funcionamiento interno del `MQTTClient`.
    fn initialize(stream: StreamType) -> Result<(Self, Receiver<PublishMessage>), Error> {
        // Más inicializaciones
        // El hilo outgoing tendrá el rx, y el hilo que lee del stream le mandará mensajes por el tx.
        // Para uso interno:
        //let (ack_tx, ack_rx) = mpsc::channel::<GenericMessage>();
        // Para devolver publish message a app:
        let (to_app_client_tx, publish_msg_to_client_rx) = mpsc::channel::<PublishMessage>();

        let mut mqtt =
            MQTTClient::mqtt_new(stream);//, ack_tx.clone());

        let mut handles = vec![];
        // Lanzo hilo para leer
        let mut self_for_read_child = mqtt.clone_refs_for_child_read()?;
        // Crea un hilo para leer desde servidor, y lo guarda para esperarlo. Hilo que lee, manda por tx a hijo que hace write.
        let h_read: JoinHandle<()> = thread::spawn(move || {
            let _res = self_for_read_child.read_from_server(&to_app_client_tx.clone());
            // []
        });
        handles.push(h_read);

        // []:
        //let self_for_write_child = mqtt.clone_refs_for_child_read()?;
        // Crea un hilo para escribir al servidor, y lo guarda para esperarlo
        let h_write: JoinHandle<()> = thread::spawn(move || {
           // let _res = self_for_write_child.write_to_server(ack_rx); // []
        });
        handles.push(h_write);
        

        mqtt.children = Some(handles);
        // Fin inicializaciones.

        Ok((mqtt, publish_msg_to_client_rx))
    }

    fn mqtt_new(
        stream: StreamType,
        //: Sender<GenericMessage>,
    ) -> Self {
        MQTTClient {
            stream,
            children: None,
            available_packet_id: 0,
            read_acks: Arc::new(Mutex::new(HashMap::new())),
            //acks_tx,
        }
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
        write_message_to_stream(&bytes_msg, &mut self.stream)?;
        println!("Mqtt publish: envío bytes publish: \n   {:?}", bytes_msg);

        Ok(publish_message)
    }

    // Nuestras apps clientes llamarán a esta función (los drones, etc)
    /// Función parte de la interfaz para uso de clientes del protocolo MQTT.
    /// Recibe el packet id, y un vector de topics a los cuales cliente desea suscribirse.
    pub fn mqtt_subscribe(
        &mut self,
        topics_to_subscribe: Vec<String>,
    ) -> Result<SubscribeMessage, Error> {
        let packet_id = self.generate_packet_id();
        println!("-----------------");
        // Construyo subscribe
        let subscribe_msg = SubscribeMessage::new(packet_id, topics_to_subscribe);
        println!("Mqtt subscribe: enviando mensaje: \n   {:?}", subscribe_msg);

        // Lo envío
        let subs_bytes = subscribe_msg.to_bytes();
        write_message_to_stream(&subs_bytes, &mut self.stream)?;
        println!(
            "Mqtt subscribe: enviado mensaje en bytes: \n   {:?}",
            subs_bytes
        );

        Ok(subscribe_msg)
    }

    /*pub fn get_publish_messages_rx(&self) -> &Option<Receiver<PublishMessage>> {
        if let Some(rx) = self.publish_msg_to_client_rx {
            &Some(rx)

        } else {
            &Some(Error::new(ErrorKind::InvalidData, ""))
        }
    }*/

    /*/// Devuelve un elemento leído, para que le llegue a cada cliente que use esta librería.
    pub fn mqtt_receive_msg_from_subs_topic(
        &self,
        //) -> Result<PublishMessage, mpsc::RecvTimeoutError> {
    ) -> Result<PublishMessage, Error> {
        // Veo si tengo el rx (hijo no lo tiene)
        if let Some(rx) = &self.publish_msg_to_client_rx {
            // Recibo un PublishMessage por el rx, para hacérselo llegar al cliente real que usa la librería
            // Leo
            match rx.recv() {
                Ok(msg) => Ok(msg),
                // (mapeo el error, por compatibilidad de tipos)
                Err(e) => 
                    Err(Error::new(ErrorKind::NotConnected, e)),
            }
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "Error: no está seteado el rx.",
            ))
        }
    }*/

    /// Envía mensaje disconnect, y cierra la conexión con el servidor.
    pub fn mqtt_disconnect(&mut self) -> Result<(), Error> {
        let msg = DisconnectMessage::new();

        // Lo envío
        let bytes_msg = msg.to_bytes();
        write_message_to_stream(&bytes_msg, &mut self.stream)?;
        println!("Mqtt disconnect: bytes {:?}", msg);

        // Cerramos la conexión con el servidor
        //if let Ok(s) = self.stream.lock() {
            match self.stream.shutdown(Shutdown::Both) {
                Ok(_) => println!("Mqtt disconnect: Conexión terminada con éxito"),
                Err(e) => println!("Mqtt disconnect: Error al terminar la conexión: {:?}", e),
            }
        //}

        Ok(())
    }

    /// Función que debe ser llamada por cada cliente que utilice la librería,
    /// como último paso, al finalizar.
    pub fn finish(&mut self) {
        if let Some(children) = self.children.take() {
            for child in children {
                let res = child.join();
                if res.is_err() {
                    println!("Mqtt cliente: error al esperar hijo de lectura.");
                }
            }
        }
    }

    /*/// Devuelve otro struct MQTTClient, con referencias a las mismas estructuras englobadas en ^Arc Mutex^
    /// que utiliza el MQTTClient para el cual se está llamando a esta función, con la diferencia de que
    /// los campos para esperar al hijo y para recibir mensajes publish están seteados en `None` ya que no son
    /// de interés para un hijo del MQTTClient original.
    fn clone_refs_for_child_read(&self) -> Self {
        Self {
            stream: self.stream.clone(),
            handle_child: None,
            rx: None,
            available_packet_id: self.available_packet_id,
            read_connack: self.read_connack.clone(),
            read_acks: self.read_acks.clone(),
        }
    }*/
    /// Devuelve otro struct MQTTClient, con referencias a las mismas estructuras englobadas en ^Arc Mutex^
    /// que utiliza el MQTTClient para el cual se está llamando a esta función, con la diferencia de que
    /// los campos para esperar al hijo y para recibir mensajes publish están seteados en `None` ya que no son
    /// de interés para un hijo del MQTTClient original.
    fn clone_refs_for_child_read(&self) -> Result<Self, Error> {
        Ok(Self {
            stream: self.stream.try_clone()?,
            children: None,
            available_packet_id: self.available_packet_id,
            read_acks: self.read_acks.clone(),
            //acks_tx: self.acks_tx.clone(),
        })
    }

    /// Función que ejecutará un hilo de MQTTClient, dedicado exclusivamente a la lectura.
    fn read_from_server(&mut self, tx: &Sender<PublishMessage>) -> Result<(), Error> {
        // Este bloque de código de acá abajo es similar a lo que hay en server,
        // pero la función que lee un mensaje lo procesa de manera diferente.

        let mut fixed_header_info: ([u8; 2], FixedHeader);
        //let (mut fixed_h_buf, mut fixed_h);
        //let ceros: &[u8; 2] = &[0; 2];
        //let mut empty: bool = false;

        //empty = &fixed_header_info.0 == ceros;
        println!("Mqtt cliente leyendo: esperando más mensajes.");
        
        loop {
            match get_fixed_header_from_stream(&mut self.stream){
                Ok(Some((fixed_h_buf, fixed_h))) => {
                        
                    fixed_header_info = (fixed_h_buf, fixed_h);
                    
                    // Caso se recibe un disconnect
                    if is_disconnect_msg(&fixed_header_info.1) {
                        shutdown(&self.stream);
                        break;
                    }

                    self.read_a_message(&fixed_header_info, tx)?; // esta función lee UN mensaje.              
            
                },
                Ok(None) => {
                    println!("Se desconectó el server.");
                    break
                },
                Err(_) => todo!(),
            }
        }




        /*while !empty {            
            // Leo fixed header para la siguiente iteración del while
            println!("Mqtt cliente leyendo: esperando más mensajes.");
            if let Some((fixed_h_buf, fixed_h)) = get_fixed_header_from_stream(&mut self.stream){}
            fixed_header_info = (fixed_h_buf, fixed_h);
            empty = &fixed_header_info.0 == ceros;

            self.read_a_message(&fixed_header_info, tx)?; // esta función lee UN mensaje.
        }*/
        Ok(())
    }

    /// Función interna que lee un mensaje, analiza su tipo, y lo procesa acorde a él.
    fn read_a_message(
        &mut self,
        fixed_header_info: &([u8; 2], FixedHeader),
        tx: &Sender<PublishMessage>,
        ) -> Result<(), Error> {
        // He leído bytes de un fixed_header, tengo que ver de qué tipo es.
        let (fixed_header_bytes, fixed_header) = fixed_header_info;
        // Soy client, siempre inicio yo la conexión, puedo recibir distintos tipos de mensaje.
        let tipo = fixed_header.get_message_type();
        let msg_bytes = get_whole_message_in_bytes_from_stream(
            fixed_header,
            &mut self.stream,
            fixed_header_bytes,
        )?;

        match tipo {
            2 => {
                // ConnAck
                println!("Mqtt cliente leyendo: recibo conn ack");
                
                // Entonces tengo el mensaje completo
                let msg = ConnackMessage::from_bytes(&msg_bytes)?; //
                println!("   Mensaje conn ack completo recibido: {:?}", msg);

            }
            3 => {
                // Publish
                println!("Mqtt cliente leyendo: RECIBO MENSAJE TIPO PUBLISH");
                // Esto ocurre cuando me suscribí a un topic, y server me envía los msjs del topic al que me suscribí
                let msg = PublishMessage::from_bytes(msg_bytes)?;
                //println!("   Mensaje publish completo recibido: {:?}", msg);

                // Le respondo un pub ack
                let _ = send_puback(&msg, &mut self.stream);

                // Envío por el channel para que le llegue al cliente real (app cliente)
                match tx.send(msg) {
                    Ok(_) => println!("Mqtt cliente leyendo: se envía por tx exitosamente."),
                    Err(_) => println!("Mqtt cliente leyendo: error al enviar por tx."),
                };
            }
            4 => {
                // PubAck
                println!("Mqtt cliente leyendo: recibo pub ack");
                
                let msg = PubAckMessage::msg_from_bytes(msg_bytes)?;
                println!("   Mensaje pub ack completo recibido: {:?}", msg);
            }
            9 => {
                // SubAck
                println!("Mqtt cliente leyendo: recibo sub ack");
                
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

    /// Devuelve el packet_id a usar para el siguiente mensaje enviado.
    /// Incrementa en 1 el atributo correspondiente, debido a la llamada anterior, y devuelve el valor a ser usado
    /// en el envío para el cual fue llamada esta función.
    fn generate_packet_id(&mut self) -> u16 {
        self.available_packet_id += 1;
        self.available_packet_id
    }
}

/// Lee un fixed header y verifica que haya sido de tipo Connack
fn read_connack(stream: &mut StreamType) -> Result<(), Error> {
    // Lee un fixed header
    let (fixed_header_buf, fixed_header) = get_fixed_header_from_stream_for_conn(stream)?;

    let fixed_header_info = (fixed_header_buf, fixed_header);

    // Verifica que haya sido de tipo Connack
    let recvd_msg_type = fixed_header_info.1.get_message_type();
    if recvd_msg_type == 2 {
        // ConnAck
        println!("Mqtt cliente leyendo: recibo conn ack");
        let recvd_bytes = get_whole_message_in_bytes_from_stream(
            &fixed_header_info.1,
            stream,
            &fixed_header_info.0,
        )?;
        // Entonces tengo el mensaje completo
        let msg = ConnackMessage::from_bytes(&recvd_bytes)?; //
        println!("   Mensaje conn ack completo recibido: {:?}", msg);
        let ret = msg.get_connect_return_code();
        if ret == ConnectReturnCode::ConnectionAccepted {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::InvalidData, ""))
        }
    } else {
        // No debería darse
        Err(Error::new(ErrorKind::InvalidData, ""))
    }
    
}