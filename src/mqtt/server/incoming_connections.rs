use std::{io::Error, net::TcpListener, result::Result, thread::JoinHandle};

use crate::{logging::string_logger::StringLogger, mqtt::stream_type::StreamType};

use super::{client_reader::ClientReader, mqtt_server::MQTTServer};

#[derive(Debug)]
pub struct ClientListener {
    logger: StringLogger,
}

impl ClientListener {
    pub fn new(logger: StringLogger) -> Self {
        ClientListener { logger }
    }

    pub fn handle_incoming_connections(
        &mut self,
        listener: TcpListener,
        mqtt_server: MQTTServer,
    ) -> Result<(), Error> {
        let mut handles = Vec::<JoinHandle<()>>::new();
        println!("Servidor iniciado. Esperando conexiones.\n");
        self.logger.log("Servidor iniciado. Esperando conexiones.".to_string());
        for stream in listener.incoming() {
            handles.push(self.handle_stream(stream?, mqtt_server.clone_ref())?);
        }

        for h in handles {
            if let Err(e) = h.join() {
                self.logger.log(format!("Error al esperar a hilo, en handle_incoming_connections: {:?}.", e));
            }
        }

        Ok(())
    }

    fn handle_stream(
        &mut self,
        mut stream: StreamType,
        mqtt_server: MQTTServer,
    ) -> Result<JoinHandle<()>, Error> {
        println!("DEBUG: CREANDO NUEVO CLIENT READER");
        self.logger.log("Creando nuevo client reader.".to_string());
        let mut client_reader = ClientReader::new(stream.try_clone()?, mqtt_server, self.logger.clone_ref())?; //

        // Hilo para cada cliente
        let logger_c = self.logger.clone_ref();
        Ok(std::thread::spawn(move || {
            if let Err(e) = client_reader.handle_client(&mut stream) {
                logger_c.log(format!("Error al esperar a hilo, en handle_stream: {:?}.", e));
            }

        }))
    }
}