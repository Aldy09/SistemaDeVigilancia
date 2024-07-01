use std::{io::{Error, Write}, sync::mpsc::Receiver};

#[derive(Debug)]
pub struct StringLoggerWriter {
    pub logger_rx: Receiver<String>,
}

impl StringLoggerWriter {
    pub fn new(logger_rx: Receiver<String>) -> Self {
        Self { logger_rx }
    }

    pub fn write_to_file(&self, message: String) -> Result<(), Error> {
        let mut file = std::fs::OpenOptions::new().append(true).open("s_log.txt")?;

        writeln!(file, "{:?}\n", message)?;

        Ok(())
    }
}
