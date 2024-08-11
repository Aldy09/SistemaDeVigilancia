use std::{io::{Error, ErrorKind}, sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender}, time::Duration};

use crate::mqtt::messages::publish_message::PublishMessage;

use super::ack_message::ACKMessage;

/// Parte interna de `MQTTClient` encargada de manejar los ack y las retransmisiones.
/// Conserva el extramo receptor de un channel (`ack_rx`).
#[derive(Debug)]
pub struct MQTTClientRetransmitter {
    ack_rx: Receiver<ACKMessage>,
}

impl MQTTClientRetransmitter {
    /// Crea y devuelve un Retransmitter, encargado de las retransmisiones, y el extremo de envío de un channel.
    pub fn new() -> (Self, Sender<ACKMessage>) {
        let (ack_tx, ack_rx) = channel::<ACKMessage>();
        (Self { ack_rx }, ack_tx)
    }

    
    // Aux: Cambiando: devuelve true si recibió el ack y false si no.
    /// Espera a que MQTTListener le informe por este rx que llegó el ack.
    /// En ese caso devuelve ok.
    /// Si eso no ocurre, debe retransmitir el mensaje original (el msg cuyo ack está esperando)
    /// hasta que llegue su ack o bien se llegue a una cantidad máxima de intentos definida como constante.
    pub fn wait_for_ack(&self, msg: &PublishMessage) -> Result<bool, Error> {
        // Extrae el packet_id
        let packet_id = msg.get_packet_id();
        if let Some(packet_id) = packet_id {
            self.start_waiting_and_check_for_ack(packet_id, msg) // Aux: #perdón no se me ocurren nombres que no sean todos iguales xd [].
        } else {
                Err(Error::new(
                ErrorKind::Other,
                "No se pudo obtener el packet id del mensaje publish",
            ))
        }
    }
    
    
    // Aux: en general, quiero, primero al leer haberme fijado el tiempo, y acá si pasó el tiempo y no recibí el ack
    // quiero volver a enviar el "msg" (obs: necesito el stream, y no quise hacerle clone otra vez entonces
    // devuelvo Ok(algo) para que signifique "sí, hay que enviarlo de nuevo" para que desde afuera llamen al writer
    // o devuelvo Ok(None) como diciendo "listo, me llegó bien el ack y no hay que hacer nada más".
    // (Podría cambiarse a devolver Result<bool, Error> capaz).
    // Aux: Cambiando: devuelve true si recibió el ack y false si no.
    fn start_waiting_and_check_for_ack(&self, packet_id: u16, _msg: &PublishMessage) -> Result<bool, Error> {
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

}