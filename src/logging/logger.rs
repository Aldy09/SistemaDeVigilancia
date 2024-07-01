use std::{io::Write, sync::mpsc::Receiver};

use crate::logging::structs_to_save_in_logger::{OperationType, StructsToSaveInLogger};

#[derive(Debug)]
pub struct Logger {
    pub logger_rx: Receiver<StructsToSaveInLogger>,
}

impl Logger {
    pub fn new(logger_rx: Receiver<StructsToSaveInLogger>) -> Self {
        Self { logger_rx }
    }

    pub fn write_in_file(&self, message: StructsToSaveInLogger) {
        let file_name = match &message {
            StructsToSaveInLogger::AppType(client_name, _, _) => match client_name.as_str() {
                "Sistema-Monitoreo" => "log_sist_monitoreo.txt",
                "Sistema-Camaras" => "log_sist_camaras.txt",
                _ => "log.txt", // Default log file
            },
            StructsToSaveInLogger::MessageType(_, _, _) => "log.txt", // Use the default log file for MessageType
        };

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(file_name)
            .unwrap();

        match message {
            StructsToSaveInLogger::AppType(client_name, app_type, operation_type) => {
                let operation_str = match operation_type {
                    OperationType::Sent => "envió",
                    OperationType::Received => "recibió",
                };
                writeln!(
                    file,
                    "{} {} un mensaje de tipo: {:?}\n",
                    client_name, operation_str, app_type
                )
                .unwrap();
            }
            StructsToSaveInLogger::MessageType(client_name, message_type, operation_type) => {
                let operation_str = match operation_type {
                    OperationType::Sent => "envió",
                    OperationType::Received => "recibió",
                };
                writeln!(
                    file,
                    "{} {} un mensaje de tipo: {:?}\n",
                    client_name, operation_str, message_type
                )
                .unwrap();
            }
        }
    }

    // pub fn write_in_file(&self, message: StructsToSaveInLogger) {
    //     let mut file = std::fs::OpenOptions::new()
    //         .append(true)
    //         .open("log.txt")
    //         .unwrap();

    //     match message {
    //         StructsToSaveInLogger::AppType(client_name, app_type, operation_type) => {
    //             let operation_str = match operation_type {
    //                 OperationType::Sent => "envió",
    //                 OperationType::Received => "recibió",
    //             };
    //             writeln!(
    //                 file,
    //                 "{} {} un mensaje de tipo: {:?}\n",
    //                 client_name, operation_str, app_type
    //             )
    //             .unwrap();
    //         }
    //         StructsToSaveInLogger::MessageType(client_name, message_type, operation_type) => {
    //             let operation_str = match operation_type {
    //                 OperationType::Sent => "envió",
    //                 OperationType::Received => "recibió",
    //             };
    //             writeln!(
    //                 file,
    //                 "{} {} un mensaje de tipo: {:?}\n",
    //                 client_name, operation_str, message_type
    //             )
    //             .unwrap();
    //         }
    //     }
    // }
}
