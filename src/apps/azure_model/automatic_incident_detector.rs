use crate::apps::incident_data::incident::Incident;
use crate::apps::incident_data::incident_source::IncidentSource;
use crate::apps::sist_camaras::camera::Camera;
use crate::apps::sist_camaras::shareable_cameras_type::ShCamerasType;
use config::{Config, File};
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use std::error::Error;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use super::api_credentials::{self, ApiCredentials};

#[derive(Debug)]
#[allow(dead_code)]
pub struct AutomaticIncidentDetector {
    cameras: ShCamerasType,
    tx: mpsc::Sender<Incident>,
    api_credentials: ApiCredentials,
}

impl AutomaticIncidentDetector {
    pub fn new(cameras: ShCamerasType, tx: mpsc::Sender<Incident>) -> Self {
        let api_credentials = ApiCredentials::new();
        Self {
            cameras,
            tx,
            api_credentials
        }
    }

    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        let (tx_fs, rx_fs) = mpsc::channel();
        let mut watcher = watcher(tx_fs, Duration::from_secs(10))?;
        watcher.watch("image_detection", RecursiveMode::Recursive)?;

        for event in rx_fs {
            match event {
                DebouncedEvent::Create(path) => {
                    if path.is_file() {
                        self.process_image(path)?;
                    }
                }
                Err(e) => println!("watch error: {:?}", e),
                _ => {}
            }
        }

        Ok(())
    }

    fn process_image(&self, image_path: PathBuf) -> Result<(), Box<dyn Error>> {
        let mut file = std::fs::File::open(&image_path)?;
        let mut buffer = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut buffer)?;

        let client = Client::new();
        let mut headers = HeaderMap::new();
        headers.insert("Prediction-Key", self.api_credentials.get_prediction_key().parse()?);
        headers.insert(CONTENT_TYPE, "application/octet-stream".parse()?);

        let res = client
            .post(&self.api_credentials.get_endpoint())
            .headers(headers)
            .body(buffer)
            .send()?;

        let res_text = res.text()?;
        let res_json: serde_json::Value = serde_json::from_str(&res_text)?;
        let incident_probability = res_json["predictions"]
            .as_array()
            .and_then(|predictions| {
                predictions.iter().find_map(|prediction| {
                    if prediction["tagName"].as_str() == Some("incidente") {
                        Some(prediction["probability"].as_f64().unwrap_or(0.0))
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(0.0);

        println!("Incident probability: {}", incident_probability);

        if incident_probability > 0.7 {
            if let Some(camera_id) = extract_camera_id(&image_path) {
                let incident_location: (f64, f64) = self.get_incident_location(camera_id);
                let incident = Incident::new(1, incident_location, IncidentSource::Automatic);
                self.tx.send(incident)?;
            } else {
                println!("Failed to extract camera ID from path");
            }
        }

        Ok(())
    }
}


fn extract_camera_id(path: &Path) -> Option<u8> {
    // Obtener el nombre del directorio padre
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|file_name| file_name.to_str())
        .and_then(|name| {
            // Supongamos que el nombre del directorio tiene el formato "camera_u8"
            // donde "u8" es el ID de la cámara.
            let prefix = "camera_";
            if name.starts_with(prefix) {
                name[prefix.len()..].parse().ok()
            } else {
                None
            }
        })
}

// Genera una ubicación de incidente aleatoria
// dentro del rango de la camara que detecto el incidente.
fn get_incident_location(&self, camera_id: u8) -> (f64, f64) {
    let camera: Camera = self.cameras.lock().unwrap().get(&camera_id).unwrap();
    let (x, y) = camera.get_position();
    let range = camera.get_range();

    let mut rng = thread_rng();

    // Genera un desplazamiento aleatorio dentro del rango para x e y
    let dx = rng.gen_range(-range..=range);
    let dy = rng.gen_range(-range..=range);

    // Calcula las nuevas coordenadas dentro del rango de la cámara
    let new_x = x + dx;
    let new_y = y + dy;

    (new_x, new_y)
}
