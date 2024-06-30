use std::{
    io::{Error, ErrorKind, Read, Write}, net::{Shutdown, TcpStream}, sync::mpsc::Sender
};

use crate::mqtt::messages::{puback_message::PubAckMessage, publish_message::PublishMessage, suback_message::SubAckMessage, subscribe_return_code::SubscribeReturnCode};
use crate::mqtt::mqtt_utils::fixed_header::FixedHeader;
type StreamType = TcpStream;


// Este archivo contiene funciones que utilizan para hacer read y write desde el stream
// tanto el message_broker_server como el mqtt_client.

// Inicio tema channel
/// Envía un mensaje de tipo PubAck al hilo outgoing, para que él lo envíe a cliente.
pub fn send_puback_to_outgoing(
    msg: &PublishMessage,
    publish_msgs_tx: Sender<Box<dyn Send>>,
) -> Result<(), Error> {
    let option_packet_id = msg.get_packet_identifier();
    let packet_id = option_packet_id.unwrap_or(0);

    let ack = PubAckMessage::new(packet_id, 0);
    //let ack_msg_bytes = ack.to_bytes();
    //write_message_to_stream(&ack_msg_bytes, stream)?;
    println!("   tipo publish: Enviando el ack: {:?}", ack);    
    if publish_msgs_tx.send(Box::new(ack)).is_err() {
        //println!("Error al enviar el PublishMessage al hilo que los procesa.");
        return Err(Error::new(
            ErrorKind::Other,
            "Error al enviar el PublishMessage al hilo que los procesa.",
        ));
    }
    Ok(())
}

/// Envía un mensaje de tipo SubAck al hilo outgoing, para que él lo envíe a cliente.
pub fn send_suback_to_outgoing(
    return_codes: Vec<SubscribeReturnCode>,
    publish_msgs_tx: Sender<Box<dyn Send>>,
) -> Result<(), Error> {
    let ack = SubAckMessage::new(0, return_codes);

    //let ack_msg_bytes = ack.to_bytes();
    //write_message_to_stream(&ack_msg_bytes, stream)?;
    println!("   tipo subscribe: Enviando el ack: {:?}", ack);
    if publish_msgs_tx.send(Box::new(ack)).is_err() {
        //println!("Error al enviar el PublishMessage al hilo que los procesa.");
        return Err(Error::new(
            ErrorKind::Other,
            "Error al enviar el PublishMessage al hilo que los procesa.",
        ));
    }

    Ok(())
}


// Fin tema channel

/*/// Envía un mensaje de tipo PubAck por el stream.
pub fn send_puback(msg: &PublishMessage, stream: &Arc<Mutex<TcpStream>>) -> Result<(), Error> {
    if let Some(packet_id) = msg.get_packet_identifier() {
        let ack = PubAckMessage::new(packet_id, 0);
        let ack_msg_bytes = ack.to_bytes();
        write_message_to_stream(&ack_msg_bytes, stream)?;
        println!("   tipo publish: Enviado el ack: {:?}", ack);
    }

    Ok(())
}*/

// Inicio funciones que manejan el stream, usadas tando por mqtt server como por client.
/// Escribe el mensaje en bytes `msg_bytes` por el stream hacia el cliente.
/// Puede devolver error si falla la escritura o el flush.
pub fn write_message_to_stream(
    msg_bytes: &[u8],
    stream: &mut StreamType,
) -> Result<(), Error> {
    // [] si hubiera un trait Message, podríamos recibir msg y hacer el msg.to_bytes() acá adentro.

    let _ = stream.write(msg_bytes)?;
    stream.flush()?;

    Ok(())
}

/// Lee `fixed_header` bytes del `stream`, sabe cuántos son por ser de tamaño fijo el fixed_header.
/// Determina el tipo del mensaje recibido que inicia por `fixed_header`.
/// Devuelve el tipo, y por cuestiones de optimización (ahorrar conversiones)
/// devuelve también fixed_header (el struct encabezado del mensaje) y fixed_header_buf (sus bytes).
pub fn get_fixed_header_from_stream(
    stream: &mut StreamType,
) -> Result<Option<([u8; 2], FixedHeader)>, Error> {
    const FIXED_HEADER_LEN: usize = FixedHeader::fixed_header_len();

    //.map_err(|_| Error::new(ErrorKind::Other, "Error al hacer un subscribe a topic"))? {
    //.map_err(|| Error::new(ErrorKind::Other, "Error al tomar lock para leer fixed header."))? {
    //let _res = stream.read(&mut fixed_header_buf);
    let res: Result<Vec<u8>, Error> = stream.bytes().take(FIXED_HEADER_LEN).collect();
    match res {
        Ok(b) if b.len() == 2 => {
            // He leído bytes de un fixed_header, tengo que ver de qué tipo es.
            let fixed_header = FixedHeader::from_bytes(b.to_vec());
            println!("READ NUEVO: Fixed header rama len 2, vale: {:?}", fixed_header);
            let fixed_header_buf = [b[0], b[1]];
        
            Ok(Some((fixed_header_buf, fixed_header)))

        },
        _ => {
            //println!("READ NUEVO: Fixed header rama None, vale: {:?}");
            Ok(None)},
    }

}

/// Una vez leídos los dos bytes del fixed header de un mensaje desde el stream,
/// lee los siguientes `remaining length` bytes indicados en el fixed header.
/// Concatena ambos grupos de bytes leídos para conformar los bytes totales del mensaje leído.
/// (Podría hacer fixed_header.to_bytes(), se aprovecha que ya se leyó fixed_header_bytes).
pub fn get_whole_message_in_bytes_from_stream(
    fixed_header: &FixedHeader,
    stream: &mut StreamType,
    fixed_header_bytes: &[u8; 2],
) -> Result<Vec<u8>, Error> {
    // Siendo que ya hemos leído fixed_header, sabemos que el resto del mensaje está disponible para ser leído.
    let msg_rem_len: usize = fixed_header.get_rem_len();
    //let rem_buf: Vec<u8> = stream.bytes().take(msg_rem_len).collect()?;
    let rem_buf: Result<Vec<u8>, Error> = stream.bytes().take(msg_rem_len).collect();

    match rem_buf {
        Ok(b) if b.len() == msg_rem_len => {
            let mut buf = fixed_header_bytes.to_vec();
            buf.extend(b);

            Ok(buf)
        },        
        _ => Err(Error::new(ErrorKind::InvalidData, "Se leyó menos de lo esperado")) // caso None o Err.
    }
}

/*/// Una vez leídos los dos bytes del fixed header de un mensaje desde el stream,
/// lee los siguientes `remaining length` bytes indicados en el fixed header.
/// Concatena ambos grupos de bytes leídos para conformar los bytes totales del mensaje leído.
/// (Podría hacer fixed_header.to_bytes(), se aprovecha que ya se leyó fixed_header_bytes).
pub fn get_whole_message_in_bytes_from_stream(
    fixed_header: &FixedHeader,
    stream: &mut StreamType,
    fixed_header_bytes: &[u8; 2],
    _msg_type_debug_string: &str,
) -> Result<Vec<u8>, Error> {
    // Instancio un buffer para leer los bytes restantes, siguientes a los de fixed header
    let msg_rem_len: usize = fixed_header.get_rem_len();
    let mut rem_buf = vec![0; msg_rem_len];
        
    let _res = stream.read(&mut rem_buf)?;

    // Ahora junto las dos partes leídas, para obt mi msg original
    let mut buf = fixed_header_bytes.to_vec();
    buf.extend(rem_buf);
    /*println!(
        "   Mensaje {} completo recibido, antes de hacerle from bytes: \n   {:?}",
        msg_type_debug_string, buf
    );*/
    //[] si hubiera un trait Message, podríamos hacerle el buf.from_bytes() acá mismo y devolver msg en vez de bytes.
    Ok(buf)
}*/

/// Recibe la primera parte del mensaje (correspondiente al fixed header),
/// lee la segunda parte y las concatena. Devuelve los bytes del mensaje completo.
pub fn complete_byte_message_read(
    stream: &mut StreamType,
    fixed_header_info: &([u8; 2], FixedHeader),
) -> Result<Vec<u8>, Error> {
    let (fixed_header_bytes, fixed_header) = fixed_header_info;

    // Lee la segunda parte del mensaje y junto ambas partes (concatena con el fixed_header)
    let msg_bytes = get_whole_message_in_bytes_from_stream(
        fixed_header,
        stream,
        fixed_header_bytes,
    )?;

    Ok(msg_bytes)
}
// Fin funciones que manejan el stream, usadas tando por mqtt server como por client.

/// Envía un mensaje de tipo PubAck por el stream.
pub fn send_puback(msg: &PublishMessage, stream: &mut TcpStream) -> Result<(), Error> {
    if let Some(packet_id) = msg.get_packet_identifier() {
        let ack = PubAckMessage::new(packet_id, 0);
        let ack_msg_bytes = ack.to_bytes();
        write_message_to_stream(&ack_msg_bytes, stream)?;
        println!("   tipo publish: Enviado el ack: {:?}", ack);
    }

    Ok(())
}

/// Devuelve si el fixed header correspondía o no al tipo de DisconnectMessage.
pub fn is_disconnect_msg(fixed_header: &FixedHeader) -> bool {
    fixed_header.get_message_type() == 14            
}

/// Cerramos la conexión por el stream recibido.
pub fn shutdown(stream: &StreamType) {
    println!("Mqtt cliente leyendo: recibo disconnect");
    match stream.shutdown(Shutdown::Both) {
        Ok(_) => println!("Conexión terminada con éxito"),
        Err(e) => println!("Error al terminar la conexión: {:?}", e),
    }
}

/// Lee `fixed_header` bytes del `stream`, sabe cuántos son por ser de tamaño fijo el fixed_header.
/// Determina el tipo del mensaje recibido que inicia por `fixed_header`.
/// Devuelve el tipo, y por cuestiones de optimización (ahorrar conversiones)
/// devuelve también fixed_header (el struct encabezado del mensaje) y fixed_header_buf (sus bytes).
pub fn get_fixed_header_from_stream_for_conn(
    stream: &mut StreamType,
) -> Result<([u8; 2], FixedHeader), Error> {
    const FIXED_HEADER_LEN: usize = FixedHeader::fixed_header_len();
    let mut fixed_header_buf: [u8; 2] = [0; FIXED_HEADER_LEN];

    // Leer
    let _res = stream.read(&mut fixed_header_buf)?;

    // He leído bytes de un fixed_header, tengo que ver de qué tipo es.
    let fixed_header = FixedHeader::from_bytes(fixed_header_buf.to_vec());

    Ok((fixed_header_buf, fixed_header))
}