use std::error::Error;
use std::fmt::Display;
//Para imprimir errores
//use std::usize;

#[derive(Debug, PartialEq)]
//enum para manejar los distintos errores al esperar un hijo hilo
pub enum BrokerErrors {
    IncommingConnectionError,
    OutgoingConnectionError,
    ConnectIsNotFirstMessageError,
    JoinIncommingThreadError,
    JoinOutgoingThreadError,
    AuthenticateError,
    SendMessageToThreadError,
    DisconnectError,
    ArgsLengthError,
    InvalidPortError,
    LinkIPAndPortError,
}

impl Error for BrokerErrors {}

impl Display for BrokerErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerErrors::IncommingConnectionError => {
                write!(f, "Error al manejar las conexiones entrantes")
            }
            BrokerErrors::OutgoingConnectionError => {
                write!(f, "Error al manejar las conexiones salientes")
            }
            BrokerErrors::ConnectIsNotFirstMessageError => {
                write!(
                    f,
                    "Error : el mensaje CONNECT no es el primero en la conexión"
                )
            }
            BrokerErrors::JoinIncommingThreadError => {
                write!(
                    f,
                    "Error al esperar a que termine el hilo de conexiones entrantes"
                )
            }
            BrokerErrors::JoinOutgoingThreadError => {
                write!(
                    f,
                    "Error al esperar a que termine el hilo de conexiones salientes"
                )
            }
            BrokerErrors::AuthenticateError => {
                write!(f, "No se pudo autenticar al cliente")
            }
            BrokerErrors::SendMessageToThreadError => {
                write!(f, "Error al enviar mensaje al hilo que los procesa")
            }
            BrokerErrors::DisconnectError => {
                write!(f, "Error al terminar la conexion")
            }
            BrokerErrors::ArgsLengthError => {
                write!(f, "Cantidad de argumentos inválida. Debe ingresar el puerto en el que desea correr el servidor.")
            }
            BrokerErrors::InvalidPortError => {
                write!(f, "El puerto ingresado no es válido")
            }
            BrokerErrors::LinkIPAndPortError => {
                write!(f, "Error al enlazar el IP con el puerto ingresado")
            }
        }
    }
}
