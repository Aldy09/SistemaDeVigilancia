use std::{
    io::{Error, Read},
    net::TcpStream,
    sync::{Arc, Mutex},
};

use crate::fixed_header::FixedHeader;

// [] AUX TEMP: HAY DOS DE ESTAS FUNCIONES QUE SE LLAMAN OLD_*, SERÁN REEMPLAZADAS POR LAS OTRAS DOS SIN "OLD_" QUE USAN ARC MUTEX
// pero todavía son necesarias xq server no tuvo refactor aún. Dsp de refactor a server las borramos a las "old_*"".
// Son exactamente iguales a las otras dos, solo cambian en que las old_ reciben un stream tcp y las nuevas un arc mutex de eso.

/// Lee `fixed_header` bytes del `stream`, sabe cuántos son por ser de tamaño fijo el fixed_header.
/// Determina el tipo del mensaje recibido que inicia por `fixed_header`.
/// Devuelve el tipo, y por cuestiones de optimización (ahorrar conversiones)
/// devuelve también fixed_header (el struct encabezado del mensaje) y fixed_header_buf (sus bytes).
//fn leer_fixed_header_de_stream_y_obt_tipo(stream: &mut TcpStream) -> Result<(u8, [u8; 2], FixedHeader), Error> {
pub fn old_leer_fixed_header_de_stream_y_obt_tipo(
    stream: &mut TcpStream,
) -> Result<[u8; 2], Error> {
    // Leer un fixed header y obtener tipo
    const FIXED_HEADER_LEN: usize = FixedHeader::fixed_header_len();
    let mut fixed_header_buf: [u8; 2] = [0; FIXED_HEADER_LEN];

    let _res = stream.read(&mut fixed_header_buf)?;

    // He leído bytes de un fixed_header, tengo que ver de qué tipo es.
    //let fixed_header = FixedHeader::from_bytes(fixed_header_buf.to_vec());
    //let tipo = fixed_header.get_tipo();

    //return Ok((tipo, fixed_header_buf, fixed_header));
    Ok(fixed_header_buf)
}

/// Una vez leídos los dos bytes del fixed header de un mensaje desde el stream,
/// lee los siguientes `remaining length` bytes indicados en el fixed header.
/// Concatena ambos grupos de bytes leídos para conformar los bytes totales del mensaje leído.
/// (Podría hacer fixed_header.to_bytes(), se aprovecha que ya se leyó fixed_header_bytes).
pub fn old_continuar_leyendo_bytes_del_msg(
    fixed_header: FixedHeader,
    stream: &mut TcpStream,
    fixed_header_bytes: &[u8; 2],
) -> Result<Vec<u8>, Error> {
    // Instancio un buffer para leer los bytes restantes, siguientes a los de fixed header
    let msg_rem_len: usize = fixed_header.get_rem_len();
    let mut rem_buf = vec![0; msg_rem_len];
    println!("UTILS AUX: REM LEN LEÍDA VALE: {}", msg_rem_len); // debug
    let _res = stream.read(&mut rem_buf)?;

    // Ahora junto las dos partes leídas, para obt mi msg original
    let mut buf = fixed_header_bytes.to_vec();
    buf.extend(rem_buf);
    println!(
        "   Mensaje en bytes recibido, antes de hacerle from_bytes: {:?}",
        buf
    );
    Ok(buf)
}

/// Lee `fixed_header` bytes del `stream`, sabe cuántos son por ser de tamaño fijo el fixed_header.
/// Determina el tipo del mensaje recibido que inicia por `fixed_header`.
/// Devuelve el tipo, y por cuestiones de optimización (ahorrar conversiones)
/// devuelve también fixed_header (el struct encabezado del mensaje) y fixed_header_buf (sus bytes).
//fn leer_fixed_header_de_stream_y_obt_tipo(stream: &mut TcpStream) -> Result<(u8, [u8; 2], FixedHeader), Error> {
pub fn leer_fixed_header_de_stream_y_obt_tipo(
    stream: &mut Arc<Mutex<TcpStream>>,
) -> Result<[u8; 2], Error> {
    // Leer un fixed header y obtener tipo
    const FIXED_HEADER_LEN: usize = FixedHeader::fixed_header_len();
    let mut fixed_header_buf: [u8; 2] = [0; FIXED_HEADER_LEN];

    {
        let mut s = stream.lock().unwrap();
        let _res = s.read(&mut fixed_header_buf)?;
    }

    // He leído bytes de un fixed_header, tengo que ver de qué tipo es.
    //let fixed_header = FixedHeader::from_bytes(fixed_header_buf.to_vec());
    //let tipo = fixed_header.get_tipo();

    //return Ok((tipo, fixed_header_buf, fixed_header));
    Ok(fixed_header_buf)
}

/// Una vez leídos los dos bytes del fixed header de un mensaje desde el stream,
/// lee los siguientes `remaining length` bytes indicados en el fixed header.
/// Concatena ambos grupos de bytes leídos para conformar los bytes totales del mensaje leído.
/// (Podría hacer fixed_header.to_bytes(), se aprovecha que ya se leyó fixed_header_bytes).
pub fn continuar_leyendo_bytes_del_msg(
    fixed_header: FixedHeader,
    stream: &mut Arc<Mutex<TcpStream>>,
    fixed_header_bytes: &[u8; 2],
) -> Result<Vec<u8>, Error> {
    // Instancio un buffer para leer los bytes restantes, siguientes a los de fixed header
    let msg_rem_len: usize = fixed_header.get_rem_len();
    let mut rem_buf = vec![0; msg_rem_len];

    {
        let mut s = stream.lock().unwrap();
        let _res = s.read(&mut rem_buf)?;
    }

    // Ahora junto las dos partes leídas, para obt mi msg original
    let mut buf = fixed_header_bytes.to_vec();
    buf.extend(rem_buf);
    println!(
        "   Mensaje en bytes recibido, antes de hacerle from_bytes: {:?}",
        buf
    );
    Ok(buf)
}
