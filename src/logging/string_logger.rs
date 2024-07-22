use std::{sync::mpsc::{self, Sender}, thread::JoinHandle};

use super::string_logger_writer::StringLoggerWriter;

#[derive(Debug)]
pub struct StringLogger {
    tx: Sender<String>,
}

impl StringLogger {
    pub fn create_logger(id: String) -> (StringLogger, JoinHandle<()>) {
        // Se crean y configuran ambos extremos del string logger
        let (string_logger_tx, string_logger_rx) = mpsc::channel::<String>();
        let logger = StringLogger::new(string_logger_tx);
        let logger_writer = StringLoggerWriter::new(id, string_logger_rx);
        let handle_logger = logger_writer.spawn_dron_stuff_to_string_logger_thread(); //

        (logger, handle_logger)
    }

    /// Extremo de envío del string logger.
    /// Es el encargado de enviar las strings a ser loggueadas.
    pub fn new(tx: Sender<String>) -> Self {
        StringLogger { tx }
    }

    // Ejemplo: logger.log(format!("Ha ocurrido un evento: {}", string_event));
    /// Función a llamar para grabar en el log el evento pasado por parámetro.
    pub fn log(&self, event: String) {
        if self.tx.send(event).is_err() {
            println!("Cliente: Error al intentar loggear.");
        }
    }

    pub fn clone_ref(&self) -> StringLogger {
        Self::new(self.tx.clone())
    }
}
