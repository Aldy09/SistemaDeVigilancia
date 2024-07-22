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
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open("s_log.txt")?;

        writeln!(file, "{}\n", message)?;

        Ok(())
    }

    /// Lanza hilo para recibir por rx cada string a logguear, y la escribe en el athcivo.
    pub fn spawn_dron_stuff_to_string_logger_thread(self
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
