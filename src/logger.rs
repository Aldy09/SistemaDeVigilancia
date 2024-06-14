use std::{io::Write, sync::mpsc::Receiver};

use crate::apps::incident::Incident;




#[derive(Debug)]
pub struct Logger {
    pub logger_rx: Receiver<Incident>,
}

impl Logger {
    pub fn new(logger_rx:Receiver<Incident>) -> Self {
        Self { logger_rx }

    }

    pub fn write_in_file(&self, incident: Incident) {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open("src/log.txt")
            .unwrap();
        let msg = format!("{:?}\n", incident);
        file.write_all(msg.as_bytes()).unwrap();
        
    }

    // pub fn get_logger_rx(&self) -> Receiver<Incident> {
    //     self.logger_rx
    // }

    // pub fn run(&self) {
    //     loop {
    //         while let Ok(msg) = self.logger_rx.recv() {
    //             match msg {
    //                 LogMessage::Info(msg) => {
    //                     info!("{}", msg);
    //                 }
    //                 LogMessage::Warn(msg) => {
    //                     warn!("{}", msg);
    //                 }
    //                 LogMessage::Error(msg) => {
    //                     error!("{}", msg);
    //                 }
    //             }
    //         }
    //     }
    // }
}