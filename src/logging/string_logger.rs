use std::{sync::mpsc::{self, Sender}, thread::JoinHandle};

use super::string_logger_writer::StringLoggerWriter;

#[derive(Debug)]
pub struct StringLogger {
    tx: Option<Sender<String>>,
}

impl StringLogger {
    /// Crea y configura todo lo necesario para utilizar el StringLogger.
    /// Devuelve el logger que posee un método de log, y un handle que debe ser esperado para terminar la ejecución correctamente.
    pub fn create_logger(id: String) -> (StringLogger, JoinHandle<()>) {
        // Se crean y configuran ambos extremos del string logger
        let (string_logger_tx, string_logger_rx) = mpsc::channel::<String>();
        let logger = StringLogger::new(string_logger_tx);
        let logger_writer = StringLoggerWriter::new(id, string_logger_rx);
        let handle_logger = logger_writer.spawn_event_listening_thread_to_write_to_file();

        (logger, handle_logger)
    }

    /// Extremo de envío del string logger.
    /// Es el encargado de enviar las strings a ser loggueadas.
    pub fn new(tx: Sender<String>) -> Self {
        Self { tx: Some(tx) }
    }

    
    // Ejemplo: logger.log(format!("Ha ocurrido un evento: {}", string_event));
    /// Función a llamar para grabar en el log el evento pasado por parámetro.
    pub fn log(&self, event: String) {
        if let Some(tx) = &self.tx{
            
            if let Err(e) = tx.send(event) {
                println!("Error al intentar loggear: {:?}.", e);
            }
        }
    }
    
    /// Función que debe ser llamada antes del final de cada programa, para no impedir la finalización del mismo.
    pub fn stop_logging(&mut self) {
        // Droppea el tx, para que se cierre el rx y el programa termine.
        self.tx = None;
    }
    
    /// Devuelve una instancia de `Self` que escribirá al mismo archivo (usa clone de su tx interno).
    pub fn clone_ref(&self) -> StringLogger {
        Self::new_for_internal_use(self.tx.clone())        
    }

    /// Para ser utilizado por clone_ref, ahora que el tx es un option para poder dropearlo con el stop_logging.
    fn new_for_internal_use(tx: Option<Sender<String>>) -> Self {
        Self { tx }
    }
}
