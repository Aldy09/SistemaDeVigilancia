use std::error::Error;
use std::fmt::Display;
//Para imprimir errores
//use std::usize;

#[derive(Debug, PartialEq)]
//enum para manejar los distintos errores al esperar un hijo hilo
pub enum CommonClientErrors {
    InvalidArgsError,
    InvalidPortError,
    ExitError,
    ReceiveExitError,
}

impl Error for CommonClientErrors {}

impl Display for CommonClientErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommonClientErrors::InvalidArgsError => {
                write!(
                    f,
                    "Cantidad de argumentos inválido. Debe ingresar: la dirección IP y 
        el puerto en el que desea correr el servidor"
                )
            }
            CommonClientErrors::InvalidPortError => {
                write!(f, "El puerto ingresado no es válido")
            }
            CommonClientErrors::ExitError => {
                write!(f, "Error al salir")
            }
            CommonClientErrors::ReceiveExitError => {
                write!(f, "Error al recibir por exit_rx")
            }
        }
    }
}
