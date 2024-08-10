use std::{sync::mpsc, thread};

use rustx::{
    apps::{
        incident_data::incident::Incident,
        sist_camaras::{
            ai_detection::ai_detector_manager::AIDetectorManager,
            manage_stored_cameras::create_cameras, types::shareable_cameras_type::ShCamerasType,
        },
    },
    logging::string_logger::StringLogger,
};

/// Este main está para llamarlo con el cargo run sin tener que levantar server monitoreo y cámaras.
fn main() {
    println!("Iniciando detector.");

    // Crea un AutomaticIncidentDetector y lo pone en funcionamiento.
    let cameras: ShCamerasType = create_cameras();
    let (tx, rx) = mpsc::channel::<Incident>();
    let (_exit_tx, exit_rx) = mpsc::channel::<()>();
    let (logger, handle_logger) = StringLogger::create_logger("detector_main".to_string());

    // Se ejecuta en otro hilo el run.
    let handle = thread::spawn(move || {
        let _ai_inc_detector = AIDetectorManager::run(cameras, tx, exit_rx, logger);
    });

    // Enviará los inc por tx, por lo que escuchamos lo recibido al rx.
    while let Ok(inc) = rx.recv() {
        println!("Se recibe por rx el inc: {:?}", inc);
        // Publicar incidente por mqtt.
    }

    // Esperar al hijo
    if handle.join().is_err() {
        println!("Error al esperar al hijo.");
    }
    // Se espera al hijo para el logger
    if handle_logger.join().is_err() {
        println!("Error al esperar al hijo para string logger writer.")
    }
}
