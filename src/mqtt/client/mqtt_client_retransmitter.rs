use std::{io::Error, sync::mpsc::{channel, Receiver, Sender}};

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

    
    /// Espera a que MQTTListener le informe por este rx que llegó el ack.
    /// En ese caso devuelve ok.
    /// Si eso no ocurre, debe retransmitir el mensaje original (el msg cuyo ack está esperando)
    /// hasta que llegue su ack o bien se llegue a una cantidad máxima de intentos definida como constante.
    pub fn wait_for_ack(&self, packet_id: u16) -> Result<(), Error> {
        for ack_message in self.ack_rx.iter() {
            if let Some(packet_identifier) = ack_message.get_packet_id() {
                if packet_id == packet_identifier {
                    println!("packet_id por parámetro {:?}", packet_id);
                    println!("   LLEGÓ EL ACK {:?}", ack_message); 
                    return Ok(());
                }
            } 
        }
        Ok(())
    }
}