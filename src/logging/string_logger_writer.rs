use std::{
    io::{Error, Write},
    sync::mpsc::Receiver, thread::{self, JoinHandle},
};

#[derive(Debug)]
pub struct StringLoggerWriter {
    pub id: String,
    pub logger_rx: Receiver<String>,
}

impl StringLoggerWriter {
    /// Crea el extremo de escritura del string logger.
    /// Es el encargado de recibir lo enviado por el otro extremo, y escribirlo a disco.
    pub fn new(id: String, logger_rx: Receiver<String>) -> Self {
        Self { id, logger_rx }
    }

    /// Escribe el mensaje recibido al archivo de log.
    fn write_to_file(&self, message: String) -> Result<(), Error> {
        
        let filename = format!("s_log_{}.txt", self.id);
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(filename)?;

        writeln!(file, "{}", message)?;

        Ok(())
    }

    /// Lanza hilo que recibe por rx cada string a logguear, y la escribe en el archivo.
    pub fn spawn_event_listening_thread_to_write_to_file(self
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            while let Ok(msg) = self.logger_rx.recv() {
                if self.write_to_file(msg).is_err() {
                    println!("LoggerWriter: error al escribir al archivo de log.");
                }
            }
        })
    }
}
