use std::error::Error;
use std::fmt::Display;
//Para imprimir errores
//use std::usize;

#[derive(Debug, PartialEq)]
//enum para manejar los distintos errores al esperar un hijo hilo
pub enum MonitoreoErrors {
    ConnectionToBrokerError,
    PublishError,
    SubscribeError,
    SendMessageToUIError,
    ReceiveMessageError,
}

impl Error for MonitoreoErrors {}

impl Display for MonitoreoErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MonitoreoErrors::ConnectionToBrokerError => {
                write!(f, "Sistema Monitoreo: Error al conectar al broker MQTT")
            }
            MonitoreoErrors::PublishError => {
                write!(f, "Sistema Monitoreo: Error al hacer un Publish")
            }
            MonitoreoErrors::SubscribeError => {
                write!(f, "Sistema Monitore: Error al hacer un Subscribe al topic")
            }
            MonitoreoErrors::SendMessageToUIError => {
                write!(f, "Sistema Monitoreo: Error al enviar mensaje a la UI")
            }
            MonitoreoErrors::ReceiveMessageError => {
                write!(
                    f,
                    "Sistema Monitoreo: Error al leer los publish messages recibidos."
                )
            }
        }
    }
}
