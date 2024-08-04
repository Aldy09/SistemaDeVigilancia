use std::io::{Error, Write};
use std::net::TcpStream;
use std::sync::mpsc::Receiver;

#[derive(Debug)]
pub struct ClientWriter {
    stream: TcpStream, // envía paquetes del TcpStream al Broker
}

impl ClientWriter {
    pub fn new(stream: TcpStream) -> Result<Self, Error> {
        Ok(ClientWriter { stream })
    }

    fn write_in(&self, stream: &mut dyn Write, packet: Vec<u8>) {
        let res = stream.write(&packet);
        match res {
            Ok(_) => println!("Packet sent: {:?} DEL LADO DE CLIENT_WRITER\n", packet),
            Err(e) => println!("Error sending packet: {:?} DEL LADO DE CLIENT_WRITER\n", e),
        }
    }

    // espera por los paquetes que recibe del Broker y los escribe en su TcpStream
    pub fn send_packets_to_client(&mut self, rx_2: Receiver<Vec<u8>>) -> Result<(), Error> {
        let mut stream = &self.stream;

        for packet in rx_2 {
            println!("Packet received: {:?} DEL LADO DE CLIENT_WRITER\n", packet);

            self.write_in(&mut stream, packet);
        }
        Ok(())
    }
}
