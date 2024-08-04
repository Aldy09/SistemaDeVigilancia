use std::io::{Error, Write};
use std::net::TcpStream;
use std::sync::mpsc::Receiver;

#[derive(Debug)]
pub struct ClientWriter {
    stream: TcpStream, // envía paquetes del TcpStream al Broker
    client_id: String,
}

impl ClientWriter {
    pub fn new(stream: TcpStream, client_id: &str) -> Result<Self, Error> {
        Ok(ClientWriter {
            stream,
            client_id: client_id.to_string(),
        })
    }

    fn write_in(&self, stream: &mut dyn Write, packet: &[u8]) {
        let res = stream.write(packet);
        let res_flush = stream.flush();
        match res {
            Ok(_) => println!("Packet sent: {:?} al cliente {:?}", packet, self.client_id),
            Err(e) => println!("Error enviando packet: {:?} al cliente {:?} \n", e, self.client_id),
        }
        match res_flush {
            Ok(_) => println!("Flushed packet al cliente {:?}", self.client_id),
            Err(e) => println!("Error flushing packet al cliente {:?} \n", e),
        }
    }

    // espera por los paquetes que recibe del Broker y los escribe en su TcpStream
    pub fn send_packets_to_client(&mut self, rx_2: Receiver<Vec<u8>>) -> Result<(), Error> {
        let mut stream = &self.stream;
        for packet in rx_2 {
            println!("Recibiendo paquete para el usuario {:?}", self.client_id);

            self.write_in(&mut stream, packet.as_slice());
        }
        Ok(())
    }
}
