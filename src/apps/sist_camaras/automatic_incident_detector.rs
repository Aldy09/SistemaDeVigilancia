use std::sync::mpsc;

use crate::apps::incident_data::incident::Incident;

use super::shareable_cameras_type::ShCamerasType;

/// Módulo de detección automática de incidentes de Sistema Cámaras.
/// Al detectar un incidente en una imagen tomada por una Cámara, lo crea y lo envía por el tx, para que el sistema cámaras pueda publicarlo por mqtt.
#[derive(Debug)]
#[allow(dead_code)]
pub struct AutomaticIncidentDetector {
    cameras: ShCamerasType,
    tx: mpsc::Sender<Incident>,
}

impl AutomaticIncidentDetector {
    pub fn new(cameras: ShCamerasType, tx: mpsc::Sender<Incident>) -> Self {
        Self { cameras, tx }
    }

    pub fn run(&self) {
        // ...
    }
}