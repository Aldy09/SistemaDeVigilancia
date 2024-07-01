use std::{
    io::{Error, Write},
    sync::mpsc::Receiver,
};

#[derive(Debug)]
pub struct StringLoggerWriter {
    pub logger_rx: Receiver<String>,
}

impl StringLoggerWriter {
    /// Crea el extremo de escritura del string logger.
    /// Es el encargado de recibir lo enviado por el otro extremo, y escribirlo a disco.
    pub fn new(logger_rx: Receiver<String>) -> Self {
        Self { logger_rx }
    }

    /// Escribe el mensaje recibido al archivo de log.
    pub fn write_to_file(&self, message: String) -> Result<(), Error> {
        let mut file = std::fs::OpenOptions::new().append(true).open("s_log.txt")?;

        writeln!(file, "{}\n", message)?;

        Ok(())
    }
}
