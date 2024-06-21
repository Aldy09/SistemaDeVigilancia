use std::error::Error;
use std::fmt::Display;
//Para imprimir errores
//use std::usize;

#[derive(Debug, PartialEq)]
//enum para manejar los distintos errores al esperar un hijo hilo
pub enum CameraErrors {
    ConnectionToBrokerError,
    PublishError,
    SubscribeToIncError,
    SendCameraToMonitoreoError,
    LockCameraError,
    InputMenuError,
    ExitMenuError,
}

impl Error for CameraErrors {}

impl Display for CameraErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CameraErrors::ConnectionToBrokerError => {
                write!(f, "Sistema Camara: Error al conectar al broker MQTT")
            }
            CameraErrors::PublishError => {
                write!(f, "Sistema Camara: Error al hacer un Publish")
            }
            CameraErrors::SubscribeToIncError => {
                write!(f, "Sistema Camara: Error al hacer un Subscribe a topic Inc")
            }
            CameraErrors::SendCameraToMonitoreoError => {
                write!(
                    f,
                    "Sistema Camara: Error al enviar la cámara del hilo abm a Sis. Monitoreo"
                )
            }
            CameraErrors::LockCameraError => {
                write!(f, "Sistema Camara: Error al tomar Lock de camaras")
            }
            CameraErrors::InputMenuError => {
                write!(
                    f,
                    "Sistema Camara: Error al tomar el input de la entrada del menú"
                )
            }
            CameraErrors::ExitMenuError => {
                write!(f, "Sistema Camara: Error al salir del menú")
            }
        }
    }
}
