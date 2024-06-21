use std::{
    io::{Error, ErrorKind, Read, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::mqtt::messages::{puback_message::PubAckMessage, publish_message::PublishMessage};
use crate::mqtt::mqtt_utils::fixed_header::FixedHeader;

// Este archivo contiene funciones que utilizan para hacer read y write desde el stream
// tanto el message_broker_server como el mqtt_client.

/// Lee `fixed_header` bytes del `stream`, sabe cuántos son por ser de tamaño fijo el fixed_header.
/// Determina el tipo del mensaje recibido que inicia por `fixed_header`.
/// Devuelve el tipo, y por cuestiones de optimización (ahorrar conversiones)
/// devuelve también fixed_header (el struct encabezado del mensaje) y fixed_header_buf (sus bytes).
pub fn get_fixed_header_from_stream(
    stream: &Arc<Mutex<TcpStream>>,
) -> Result<([u8; 2], FixedHeader), Error> {
    const FIXED_HEADER_LEN: usize = FixedHeader::fixed_header_len();
    let mut fixed_header_buf: [u8; 2] = [0; FIXED_HEADER_LEN];

    // Tomo lock y leo del stream
    if let Ok(mut s) = stream.lock() {
        // Si nadie me envía mensaje, no quiero bloquear en el read con el lock tomado, quiero soltar el lock
        let set_read_timeout = s.set_read_timeout(Some(Duration::from_millis(300)));
        match set_read_timeout {
            Ok(_) => {
                // Leer
                let _res = s.read(&mut fixed_header_buf)?;
                // Unset del timeout, ya que como hubo fixed header, es 100% seguro que seguirá el resto del mensaje
                let _ = s.set_read_timeout(None);
            }
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut {
                    // Este tipo de error es esperable, se utiliza desde afuera
                    return Err(Error::new(ErrorKind::Other, "No se leyó."));
                } else {
                    // Rama solamente para debugging, éste es un error real
                    println!("OTRO ERROR, que no fue de timeout: {:?}", e);
                    return Err(Error::new(ErrorKind::Other, "OTRO ERROR"));
                }
            }
        }
    }

    // He leído bytes de un fixed_header, tengo que ver de qué tipo es.
    let fixed_header = FixedHeader::from_bytes(fixed_header_buf.to_vec());

    Ok((fixed_header_buf, fixed_header))
}

/// Una vez leídos los dos bytes del fixed header de un mensaje desde el stream,
/// lee los siguientes `remaining length` bytes indicados en el fixed header.
/// Concatena ambos grupos de bytes leídos para conformar los bytes totales del mensaje leído.
/// (Podría hacer fixed_header.to_bytes(), se aprovecha que ya se leyó fixed_header_bytes).
pub fn get_whole_message_in_bytes_from_stream(
    fixed_header: &FixedHeader,
    stream: &Arc<Mutex<TcpStream>>,
    fixed_header_bytes: &[u8; 2],
    _msg_type_debug_string: &str,
) -> Result<Vec<u8>, Error> {
    // Instancio un buffer para leer los bytes restantes, siguientes a los de fixed header
    let msg_rem_len: usize = fixed_header.get_rem_len();
    let mut rem_buf = vec![0; msg_rem_len];
    // Tomo lock y leo del stream
    {
        if let Ok(mut s) = stream.lock() {
            // si uso un if let, no nec el scope de afuera para dropear []
            let _res = s.read(&mut rem_buf)?;
        }
    }
    // Ahora junto las dos partes leídas, para obt mi msg original
    let mut buf = fixed_header_bytes.to_vec();
    buf.extend(rem_buf);
    /*println!(
        "   Mensaje {} completo recibido, antes de hacerle from bytes: \n   {:?}",
        msg_type_debug_string, buf
    );*/
    //[] si hubiera un trait Message, podríamos hacerle el buf.from_bytes() acá mismo y devolver msg en vez de bytes.
    Ok(buf)
}

/// Escribe el mensaje en bytes `msg_bytes` por el stream hacia el cliente.
/// Puede devolver error si falla la escritura o el flush.
pub fn write_message_to_stream(
    msg_bytes: &[u8],
    stream: &Arc<Mutex<TcpStream>>,
) -> Result<(), Error> {
    // [] si hubiera un trait Message, podríamos recibir msg y hacer el msg.to_bytes() acá adentro.
    if let Ok(mut s) = stream.lock() {
        let _ = s.write(msg_bytes)?;
        s.flush()?;
    } else {
        return Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock para hacer write.",
        ));
    }
    Ok(())
}

/// Envía un mensaje de tipo PubAck por el stream.
pub fn send_puback(msg: &PublishMessage, stream: &Arc<Mutex<TcpStream>>) -> Result<(), Error> {
    if let Some(packet_id) = msg.get_packet_identifier() {
        let ack = PubAckMessage::new(packet_id, 0);
        let ack_msg_bytes = ack.to_bytes();
        write_message_to_stream(&ack_msg_bytes, stream)?;
        println!("   tipo publish: Enviado el ack: {:?}", ack);
    }

    Ok(())
}
