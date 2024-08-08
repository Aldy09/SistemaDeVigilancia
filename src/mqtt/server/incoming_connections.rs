
use std::io::Error;
use std::net::TcpListener;

use std::result::Result;
//use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;

use crate::mqtt::stream_type::StreamType;

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
        let mut handles = Vec::<JoinHandle<()>>::new();
        println!("Servidor iniciado. Esperando conexiones.\n");
        for stream in listener.incoming() {
            handles.push(self.handle_stream(stream?, mqtt_server.clone_ref())?);
        }

        for h in handles {
            let _ = h.join();
        }

        Ok(())
    }

    fn handle_stream(
        &mut self,
        mut stream: StreamType,
        mqtt_server: MQTTServer,
    ) -> Result<JoinHandle<()>, Error> {
        println!("DEBUG: CREANDO NUEVO CLIENT READER");
        let mut client_reader = ClientReader::new(stream.try_clone()?, mqtt_server)?; // Aux: sí, sí, ya sé que queremos SACAR los try_clone, es para que siga compilando mientras hay refactor.

        // Hilo para cada cliente
        Ok(std::thread::spawn(move || {
            let _ = client_reader.handle_client(&mut stream);
        }))
    }
}


impl Default for ClientListener {
    fn default() -> Self {
        Self::new()
    }
}