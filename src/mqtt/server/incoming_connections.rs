use std::{io::Error, net::TcpListener, result::Result, thread::JoinHandle};

use crate::mqtt::stream_type::StreamType;

use super::{client_reader::ClientReader, mqtt_server::MQTTServer};

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
        let mut client_reader = ClientReader::new(stream.try_clone()?, mqtt_server)?; //

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
