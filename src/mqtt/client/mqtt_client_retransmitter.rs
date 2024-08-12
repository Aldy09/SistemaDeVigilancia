use std::{io::{Error, ErrorKind}, net::Shutdown, sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender}, time::Duration};

use crate::mqtt::{messages::{disconnect_message::DisconnectMessage, message::Message, packet_type::PacketType, publish_message::PublishMessage}, mqtt_utils::utils::write_message_to_stream};

use super::{ack_message::ACKMessage, mqtt_client::ClientStreamType};

/// Parte interna de `MQTTClient` encargada de manejar los ack y las retransmisiones.
/// Conserva el extramo receptor de un channel (`ack_rx`).
#[derive(Debug)]
pub struct MQTTClientRetransmitter {
    ack_rx: Receiver<ACKMessage>,
    stream: ClientStreamType,
}

impl MQTTClientRetransmitter {
    /// Crea y devuelve un Retransmitter, encargado de las retransmisiones, y el extremo de envío de un channel.
    pub fn new(stream: ClientStreamType) -> (Self, Sender<ACKMessage>) {
        let (ack_tx, ack_rx) = channel::<ACKMessage>();
        (Self { ack_rx , stream }, ack_tx)
    }
    
    /// Envía el mensaje `msg` recibido una vez, espera por el ack, y si es necesario lo retransmite una cierta
    /// cantidad de veces.
    pub fn send_and_retransmit<T: Message>(&mut self, msg: &T) -> Result<(), Error> {
        self.send_msg(msg.to_bytes())?;
        if let Err(e) = self.wait_for_ack_wrapper(msg) {
            println!("Error al esperar ack del publish: {:?}", e);
        };
        Ok(())
    }

    // Aux: esta función estaba afuera en el MQTTClient.
    // Si no se pudo esperar el ack, se deberia reintentar el publish
    /// Espera a recibir el ack para el packet_id del mensaje `msg`.
    fn wait_for_ack_wrapper<T: Message>(&mut self, msg: &T) -> Result<(), Error> {
        match msg.get_type() {
            // Si es publish, ver el qos
            PacketType::Publish => {
                if let Some(pub_msg) = msg.as_any().downcast_ref::<PublishMessage>() {
                    let qos = pub_msg.get_qos();
                    if qos == 1 {
                        return self.wait_and_retransmit(pub_msg);
                    } else {
                        return Ok(());
                    }
                }
            }
            PacketType::Subscribe => {
                return self.wait_and_retransmit(msg);
            }
            _ => {}
        }

        Ok(())
    }

    // Aux: esta función estaba afuera en MQTTClient.
    fn wait_and_retransmit<T: Message>(&mut self, msg: &T) -> Result<(), Error> {
        let packet_id = msg.get_packet_id();
        // Espero la primera vez, para el publish que hicimos arriba. Si se recibió ack, no hay que hacer nada más.
        let mut received_ack = self.has_ack_arrived(packet_id)?;
        if received_ack {
            return Ok(());
        }

        // No recibí ack, entonces tengo que continuar retransmitiendo, hasta un máx de veces.
        const AMOUNT_OF_RETRIES: u8 = 5; // cant de veces que va a reintentar, hasta que desista y dé error.
        let mut remaining_retries = AMOUNT_OF_RETRIES;
        while remaining_retries > 0 {
            // Si el Retransmitter determina que se debe volver a enviar el mensaje, lo envío.
            if !received_ack {
                self.send_msg(msg.to_bytes())?
            } else {
                break;
            }
            received_ack = self.has_ack_arrived(packet_id)?;

            remaining_retries -= 1; // Aux: sí, esto podría ser un for. Se puede cambiar.
        }

        if !received_ack {
            // Ya salí del while, retransmití muchas veces y nunca recibí el ack, desisto
            return Err(Error::new(
                ErrorKind::Other,
                "MAXRETRIES, se retransmitió sin éxito.",
            ));
        }

        Ok(())
    }



    
    

    
    // Aux: Cambiando: devuelve true si recibió el ack y false si no.
    /// Espera a que MQTTListener le informe por este rx que llegó el ack.
    /// En ese caso devuelve ok.
    /// Si eso no ocurre, debe retransmitir el mensaje original (el msg cuyo ack está esperando)
    /// hasta que llegue su ack o bien se llegue a una cantidad máxima de intentos definida como constante.
    fn has_ack_arrived(&self, packet_id: Option<u16>) -> Result<bool, Error> {
        // Extrae el packet_id
        if let Some(packet_id) = packet_id {
            self.start_waiting_and_check_for_ack(packet_id) // Aux: #perdón no se me ocurren nombres que no sean todos iguales xd [].
        } else {
                Err(Error::new(
                ErrorKind::Other,
                "No se pudo obtener el packet id del mensaje publish",
            ))
        }
    }
    
    // Aux: en general, quiero, primero al leer haberme fijado el tiempo, y acá si pasó el tiempo y no recibí el ack
    // quiero volver a enviar el "msg" (obs: necesito el stream, y no quise hacerle clone otra vez entonces
    // devuelvo Ok(false) para que signifique "sí, hay que enviarlo de nuevo" para que desde afuera llamen al writer
    // o devuelvo Ok(true) como diciendo "listo, me llegó bien el ack y no hay que hacer nada más".
    // (Podría cambiarse a devolver Result<bool, Error> capaz).
    // Aux: Cambiando: devuelve true si recibió el ack y false si no.
    fn start_waiting_and_check_for_ack(&self, packet_id: u16) -> Result<bool, Error> {
        // Comentar una y descomentar la otra, para probar
        // Versión lo que había, sin esperar un tiempo
        //self.aux_version_vieja(packet_id)
        
        // Versión nueva, esperando como máx un tiempo para que si no se recibió se retransmita:
        self.aux_version_nueva(packet_id)
    }
    
    // La versión nueva
    fn aux_version_nueva(&self, packet_id: u16) -> Result<bool, Error> {
        // Leo esperando un cierto tiempo, si en el período [0, ese tiempo) no me llega el ack, lo quiero retransmitir.
        const ACK_WAITING_INTERVAL: u64 = 1000; // Aux: Fijarse un número que tenga sentido.
        match self.ack_rx.recv_timeout(Duration::from_millis(ACK_WAITING_INTERVAL)){
            Ok(ack_message) => {
                // Se recibió el ack
                if let Some(packet_identifier) = ack_message.get_packet_id() {
                    if packet_id == packet_identifier {
                        println!("packet_id por parámetro {:?}", packet_id);
                        println!("   LLEGÓ EL ACK {:?}", ack_message); 
                        return Ok(true);
                    }
                }
            },
            Err(e) => {
                match e {
                    RecvTimeoutError::Timeout => {
                        // Se cumplió el tiempo y el ack No se recibió.
                        return Ok(false);

                    },
                    RecvTimeoutError::Disconnected => {
                        // Se cerró el channel. Termina el programa.
                        // Ver.
                    },
                }
            },
        }
        Ok(false)
    }
    
    // La versión que había
    fn _aux_version_vieja(&self, packet_id: u16) -> Result<bool, Error> {
        for ack_message in self.ack_rx.iter() { // Aux: si es de a uno, un if andaría
            if let Some(packet_identifier) = ack_message.get_packet_id() {
                if packet_id == packet_identifier {
                    println!("packet_id por parámetro {:?}", packet_id);
                    println!("   LLEGÓ EL ACK {:?}", ack_message); 
                    return Ok(true);
                }
            } 
        }
        Ok(false)
    }

    // Función relacionada con el Retransmitter:
    /// Función para ser usada por `MQTTClient`, cuando el `Retransmitter` haya determinado que el `msg` debe
    /// enviarse por el stream a server.
    fn send_msg(&mut self, bytes_msg: Vec<u8>) -> Result<(), Error> {
        write_message_to_stream(&bytes_msg, &mut self.stream)?;
        Ok(())
    }
    
    /// Envía el mensaje disconnect recibido por parámetro y cierra la conexión.
    pub fn send_and_shutdown_stream(&mut self, msg: DisconnectMessage) -> Result<(), Error> {
        self.send_msg(msg.to_bytes());
        // Cerramos la conexión con el servidor
        self.stream.shutdown(Shutdown::Both)?; // Aux: mover esto a alguien que tenga el stream
        Ok(())
    }
    

}