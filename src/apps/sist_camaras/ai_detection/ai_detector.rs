use rand::{thread_rng, Rng};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use std::error::Error;
use std::io::ErrorKind;
use std::sync::mpsc;

use crate::apps::sist_camaras::shareable_cameras_type::ShCamerasType;
use crate::apps::incident_data::incident::Incident;
use crate::apps::incident_data::incident_source::IncidentSource;
use super::api_credentials::ApiCredentials;

/// Se encarga de comunicarse con el proveedor de inteligencia artificial, enviarle la
/// imagen de la cámara y evaluar si la respuesta indica que la imagen contiene o no un incidente.
/// En caso afirmativo envía por el tx el Incidente (Sistema Cámaras lo publicará por MQTT).
#[derive(Debug)]
pub struct AutomaticIncidentDetector {
    cameras: ShCamerasType,
    tx: mpsc::Sender<Incident>,
    last_incident_id: u8,
}

impl AutomaticIncidentDetector {
    pub fn new(cameras: ShCamerasType, tx: mpsc::Sender<Incident>) -> Self {
        Self {
            cameras,
            tx,
            last_incident_id: 0,
        }
    }

    pub fn clone_refs(&self) -> Self {
        Self {
            cameras: self.cameras.clone(),
            tx: self.tx.clone(),
            last_incident_id: self.last_incident_id,
        }
    }

    /// Lee la imagen de `image_path`, se la envía al proveedor de ia y analiza su respuesta para concluir si
    /// la imagen contiene o no un incidente. En caso afirmativo, se procesa al incidente.
    pub fn process_image(&mut self, image: Vec<u8>, cam_id: u8) -> Result<(), Box<dyn Error>> {
        let api_credentials = ApiCredentials::new();
        
        let (client, headers) = create_client_and_headers(&api_credentials)?;

        // Se envía la imagen al proveedor
        let res = client
            .post(api_credentials.get_endpoint())
            .headers(headers)
            .body(image)
            .send()?;

        let res_text = res.text()?;
        let incident_probability = process_response(&res_text)?;

        println!("Probability: {:?}", incident_probability);
        if incident_probability > 0.7 {
            self.process_incident(cam_id)?;
        }

        Ok(())
    }

    /// Recibe el image_path de la imagen en la que se detectó un incidente, crea el Incident y lo envía internamente para
    /// ser publicado por MQTT.
    fn process_incident(&mut self, cam_id: u8) -> Result<(), Box<dyn Error>>{
        // obtenemos la posición
        println!("camera_id: {}", cam_id);
        let incident_position: (f64, f64) = self.get_incident_position(cam_id)?;
        // creamos el incidente
        let inc_id = self.get_next_incident_id();
        let incident = Incident::new(inc_id, incident_position, IncidentSource::Automated);
        println!("Incidente creado! {:?}", incident);
        // se envía el inc para ser publicado
        self.tx.send(incident)?;
        Ok(())
    }

    // Genera una ubicación de incidente aleatoria
    // dentro del rango de la camara que detectó el incidente.
    fn get_incident_position(&self, camera_id: u8) -> Result<(f64, f64), std::io::Error> {
        if let Ok(cameras) = self.cameras.lock(){
            if let Some(camera) = cameras.get(&camera_id) {

                let (x, y) = camera.get_position();
                let range = camera.get_range_area();

                let mut rng = thread_rng();

                // Genera un desplazamiento aleatorio dentro del rango para x e y
                let dx = rng.gen_range(0.0..=range);
                let dy = rng.gen_range(0.0..=range);

                // Calcula las nuevas coordenadas dentro del rango de la cámara
                let new_x = x + dx as f64;
                let new_y = y + dy as f64;

                return Ok((new_x, new_y));
            }
        }

        Err(std::io::Error::new(ErrorKind::Other, "Error al obtener la camera del hashmap en get_incident_position."))
        
    }

    fn get_next_incident_id(&mut self) -> u8 {
        self.last_incident_id += 1;
        self.last_incident_id
    }

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



/// Interpreta el res_text recibido como json y devuelve la probabilidad con que el mismo afirma que
/// se trata de un incidente.
fn process_response(res_text: &str) -> Result<f64, Box<dyn Error>> {
    let res_json: serde_json::Value = serde_json::from_str(res_text)?;
    let incident_probability_option  = res_json["predictions"]
        .as_array()
        .and_then(|predictions| {
            predictions.iter().find_map(|prediction| {
                if prediction["tagName"].as_str() == Some("incidente") {
                    prediction["probability"].as_f64()
                } else {
                    None
                }
            })
        });
    
    if let Some(incident_probability) = incident_probability_option {
        Ok(incident_probability)
    } else {
        Err(Box::new(std::io::Error::new(ErrorKind::Other, "Error al obtener la incident_probability.")))
    }
}

