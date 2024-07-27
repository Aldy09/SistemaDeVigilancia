

use notify::event::EventKind;
use notify::{RecursiveMode, Watcher};
use rayon::ThreadPoolBuilder;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use crate::apps::incident_data::incident::Incident;


use crate::apps::incident_data::incident_source::IncidentSource;
use crate::apps::sist_camaras::shareable_cameras_type::ShCamerasType;

use super::api_credentials::ApiCredentials;

#[derive(Debug)]
#[allow(dead_code)]
pub struct AutomaticIncidentDetector {
    cameras: ShCamerasType,
    tx: mpsc::Sender<Incident>,
}

impl AutomaticIncidentDetector {
    pub fn new(cameras: ShCamerasType, tx: mpsc::Sender<Incident>) -> Self {
        Self {
            cameras,
            tx,
        }
    }

    pub fn clone_ref(&self) -> Self {
        Self {
            cameras: self.cameras.clone(),
            tx: self.tx.clone(),
        }
    }

    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        let (tx_fs, rx_fs) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx_fs)?;
        println!("por crear el path");
        let path = Path::new("./src/apps/sist_camaras/azure_model/image_detection");
        watcher.watch(path, RecursiveMode::Recursive)?;
        // Crear un pool de threads con el número de threads deseado
        let pool = ThreadPoolBuilder::new().num_threads(6).build().unwrap();

        for event in rx_fs {
            let self_clone = self.clone_ref();
            match event {
                Ok(event) => match event.kind {
                    EventKind::Create(_) => {
                        if let Some(path) = event.paths.first() {
                            if path.is_file() {
                                let image_path = path.clone(); // Clona la ruta para moverla al hilo
                                // Lanza un hilo por cada imagen a procesar
                                pool.spawn(move || {
                                    process_image(image_path, &self_clone).unwrap();
                                });
                            }
                        }
                    }
                    _ => {} // Ignorar otros eventos
                },
                Err(e) => println!("watch error: {:?}", e),
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


fn process_image(image_path: PathBuf, self_clone: &AutomaticIncidentDetector) -> Result<(), Box<dyn Error>> {
    let buffer = read_image(&image_path)?;
    let api_credentials = ApiCredentials::new();
    let (client, headers) = create_client_and_headers(&api_credentials)?;

    let res = client
        .post(api_credentials.get_endpoint())
        .headers(headers)
        .body(buffer)
        .send()?;

    let res_text = res.text()?;
    let incident_probability = process_response(&res_text)?;

    if incident_probability > 0.7 {
        process_incident(image_path, self_clone);
    }

    Ok(())
}

fn process_incident(image_path: PathBuf, self_clone: &AutomaticIncidentDetector) -> () {
    if let Some(_camera_id) = extract_camera_id(&image_path) {
        //let incident_location: (f64, f64) = get_incident_location(camera_id);
        let incident_location: (f64, f64) = (-34.6005, -58.3846);
        let incident = Incident::new(1, incident_location, IncidentSource::Automated);
        self_clone.tx.send(incident).unwrap();
    } else {
        println!("Failed to extract camera ID from path");
    }

}

// Genera una ubicación de incidente aleatoria
    // dentro del rango de la camara que detecto el incidente.
    // fn get_incident_location(&self, camera_id: u8) -> (f64, f64) {
    //     let camera: = self.cameras.lock().unwrap().get(&camera_id).unwrap();
    //     let (x, y) = camera.get_position();
    //     let range = camera.get_range();

    //     let mut rng = thread_rng();

    //     // Genera un desplazamiento aleatorio dentro del rango para x e y
    //     let dx = rng.gen_range(-range..=range);
    //     let dy = rng.gen_range(-range..=range);

    //     // Calcula las nuevas coordenadas dentro del rango de la cámara
    //     let new_x = x + dx;
    //     let new_y = y + dy;

    //     (new_x, new_y)
    // }


fn read_image(image_path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut file = std::fs::File::open(image_path)?;
    let mut buffer = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buffer)?;
    Ok(buffer)
}

fn create_client_and_headers(api_credentials: &ApiCredentials) -> Result<(Client, HeaderMap), Box<dyn Error>> {
    let client = Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        "Prediction-Key",
        api_credentials.get_prediction_key().parse()?,
    );
    headers.insert(CONTENT_TYPE, "application/octet-stream".parse()?);
    Ok((client, headers))
}




fn process_response(res_text: &str) -> Result<f64, Box<dyn Error>> {
    let res_json: serde_json::Value = serde_json::from_str(res_text)?;
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
    Ok(incident_probability)
}

