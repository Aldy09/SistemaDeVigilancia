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

type StreamType = TcpStream;

#[derive(Debug)]
pub struct MQTTClient {
    stream: StreamType,
    publish_msg_to_client_tx: Sender<PublishMessage>,
}

impl MQTTClient {

    pub fn new(publish_msg_to_client_tx: Sender<PublishMessage>, stream: StreamType) -> Self {
        MQTTClient {
            
        }
    }
    

    /// Realiza las inicializaciones necesarias para el funcionamiento interno del `MQTTClient`.
    fn initialize(stream: StreamType) -> Result<Self, Error> {
    

        let mut mqtt =
            MQTTClient::mqtt_new(stream, publish_msg_to_client_rx);//, ack_tx.clone());

        let mut handles = vec![];
        // Lanzo hilo para leer
        let mut self_for_read_child = mqtt.clone_refs_for_child_read()?;
        // Crea un hilo para leer desde servidor, y lo guarda para esperarlo. Hilo que lee, manda por tx a hijo que hace write.
        let h_read: JoinHandle<()> = thread::spawn(move || {
            let _res = self_for_read_child.read_from_server(&to_app_client_tx.clone());
            // []
        });
        handles.push(h_read);

        mqtt.children = Some(handles);
        // Fin inicializaciones.

        Ok(mqtt)
    }

    fn mqtt_new(
        stream: StreamType,
        publish_msg_to_client_tx: Sender<PublishMessage>,
    ) -> Self {
        MQTTClient {
            stream,
            children: None,
            publish_msg_to_client_tx,
            available_packet_id: 0,
            read_acks: Arc::new(Mutex::new(HashMap::new())),
        }
    }



    
    /// Devuelve un elemento leído, para que le llegue a cada cliente que use esta librería.
    pub fn mqtt_receive_msg_from_subs_topic(
        &self,
        //) -> Result<PublishMessage, mpsc::RecvTimeoutError> {
    ) -> Result<PublishMessage, Error> {
        // Veo si tengo el rx (hijo no lo tiene)
        if let Some(rx) = &self.publish_msg_to_client_rx {
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

   
    /// Devuelve otro struct MQTTClient, con referencias a las mismas estructuras englobadas en ^Arc Mutex^
    /// que utiliza el MQTTClient para el cual se está llamando a esta función, con la diferencia de que
    /// los campos para esperar al hijo y para recibir mensajes publish están seteados en `None` ya que no son
    /// de interés para un hijo del MQTTClient original.
    fn clone_refs_for_child_read(&self) -> Result<Self, Error> {
        Ok(Self {
            stream: self.stream.try_clone()?,
            children: None,
            publish_msg_to_client_rx: None,
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
                Ok(None) => {},
                Err(_) => todo!(),
            }
        }

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


}


