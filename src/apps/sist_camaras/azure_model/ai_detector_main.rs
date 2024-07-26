use std::{sync::mpsc, thread};

use rustx::apps::{incident_data::incident::Incident, sist_camaras::{azure_model::automatic_incident_detector::AutomaticIncidentDetector, manage_stored_cameras::create_cameras, shareable_cameras_type::ShCamerasType}};

fn main() {

    // Crea un AutomaticIncidentDetector y lo pone en funcionamiento.
    let cameras: ShCamerasType = create_cameras();
    let (tx, rx) = mpsc::channel::<Incident>();

    // Se ejecuta en otro hilo el run.
    let handle = thread::spawn(move || {
        let ai_inc_detector = AutomaticIncidentDetector::new(cameras, tx);
        match ai_inc_detector.run() {
            Ok(_) => println!("Finalizado con éxito."),
            Err(e) => println!("Error en el detector: {:?}.", e),
        }
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
}