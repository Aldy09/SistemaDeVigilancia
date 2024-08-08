use std::sync::mpsc;

type Channels = (
    mpsc::Sender<Vec<u8>>,
    mpsc::Receiver<Vec<u8>>,
    mpsc::Sender<bool>,
    mpsc::Receiver<bool>,
);

/// Función que crea y devuelve extremos de channels para Sistema Cámaras.
pub fn create_channels() -> Channels {
    let (cameras_tx, cameras_rx) = mpsc::channel::<Vec<u8>>();
    let (exit_tx, exit_rx) = mpsc::channel::<bool>();
    (cameras_tx, cameras_rx, exit_tx, exit_rx)
}