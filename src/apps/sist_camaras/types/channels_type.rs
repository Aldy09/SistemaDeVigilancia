use std::sync::mpsc::{self, Receiver, Sender};

type Channels = (
    Sender<Vec<u8>>,
    Receiver<Vec<u8>>,
    Sender<bool>,
    Receiver<bool>,
    Sender<()>,
    Receiver<()>,
);

/// Función que crea y devuelve extremos de channels para Sistema Cámaras.
pub fn create_channels() -> Channels {
    // ABM y CamerasLogic envían una camera en bytes por tx para que hilo las publique por MQTT
    let (cameras_tx, cameras_rx) = mpsc::channel::<Vec<u8>>();
    // ABM en su opción `4 _ Salir` envía aviso por tx para que hilo de Exit que escucha 'salga' (envía MQTT disconnect)
    let (exit_tx, exit_rx) = mpsc::channel::<bool>();
    // Hilo de Exit cuando recibe aviso, lo propaga por tx hacia el Detector para que él corte su loop
    let (exit_detector_tx, exit_detector_rx) = mpsc::channel::<()>();
    (cameras_tx, cameras_rx, exit_tx, exit_rx, exit_detector_tx, exit_detector_rx)
}