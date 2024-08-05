use std::io::Error;
use std::net::{TcpListener, TcpStream};

use std::result::Result;
//use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;

use super::client_reader::ClientReader;
use super::mqtt_server::MQTTServer;

#[derive(Debug)]
pub struct ClientListener {}

impl ClientListener {
    pub fn new() -> Self {
        ClientListener {}
    }

    pub fn handle_incoming_connections(
        &mut self,
        listener: TcpListener,
        mqtt_server: MQTTServer,
    ) -> Result<(), Error> {
        let mut handlers = Vec::<JoinHandle<()>>::new();
        println!("Servidor iniciado. Esperando conexiones.\n");
        for stream in listener.incoming() {
            handlers.push(self.handle_stream(stream?, mqtt_server.clone_ref())?);
        }

        for h in handlers {
            let _ = h.join();
        }

        Ok(())
    }

    fn handle_stream(
        &mut self,
        stream: TcpStream,
        mqtt_server: MQTTServer,
    ) -> Result<JoinHandle<()>, Error> {
        println!("DEBUG: CREANDO NUEVO CLIENT READER");
        let mut client_reader = ClientReader::new(stream, mqtt_server)?;

        // Hilo para cada cliente
        Ok(std::thread::spawn(move || {
            let _ = client_reader.handle_client();
        }))
    }
}
