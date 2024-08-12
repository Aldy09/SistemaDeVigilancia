use rand::{thread_rng, Rng};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use std::error::Error;
use std::io::ErrorKind;
use std::sync::{mpsc, Arc, Mutex};

use crate::apps::sist_camaras::types::shareable_cameras_type::ShCamerasType;
use crate::apps::incident_data::incident::Incident;
use crate::apps::incident_data::incident_source::IncidentSource;
use crate::logging::string_logger::StringLogger;
use super::api_credentials::ApiCredentials;
use super::properties::DetectorProperties;

/// Se encarga de comunicarse con el proveedor de inteligencia artificial, enviarle la
/// imagen de la cámara y evaluar si la respuesta indica que la imagen contiene o no un incidente.
/// En caso afirmativo envía por el tx el Incidente (Sistema Cámaras lo publicará por MQTT).
#[derive(Debug)]
pub struct AutomaticIncidentDetector {
    cameras: ShCamerasType,
    tx: mpsc::Sender<Incident>,
    last_incident_id: Arc<Mutex<u8>>,
    properties: DetectorProperties,
    logger: StringLogger,
}

impl AutomaticIncidentDetector {
    pub fn new(cameras: ShCamerasType, tx: mpsc::Sender<Incident>, properties: DetectorProperties, logger: StringLogger) -> Self {
        Self {
            cameras,
            tx,
            last_incident_id: Arc::new(Mutex::new(0)),
            properties,
            logger,
        }
    }

    pub fn clone_refs(&self) -> Self {
        Self {
            cameras: self.cameras.clone(),
            tx: self.tx.clone(),
            last_incident_id: self.last_incident_id.clone(),
            properties: self.properties.clone(),
            logger: self.logger.clone_ref(),
        }
    }

    /// Lee la imagen de `image_path`, se la envía al proveedor de ia y analiza su respuesta para concluir si
    /// la imagen contiene o no un incidente. En caso afirmativo, se procesa al incidente.
    pub fn process_image(&mut self, image: Vec<u8>, cam_id: u8) -> Result<(), Box<dyn Error>> {
        let api_credentials = ApiCredentials::new(self.properties.get_api_credentials_file_path());
        
        let (client, headers) = create_client_and_headers(&api_credentials)?;

        // Se envía la imagen al proveedor
        let res = client
            .post(api_credentials.get_endpoint())
            .headers(headers)
            .body(image)
            .send()?;

        let res_text = res.text()?;
        let incident_probability = self.process_response(&res_text)?;

        println!("Detector: Probability: {:?}", incident_probability);
        self.logger.log(format!("Detector: Probability: {:?}", incident_probability));
        if incident_probability > self.properties.get_inc_threshold() {
            self.process_incident(cam_id)?;
        }

        Ok(())
    }

    /// Interpreta el res_text recibido como json y devuelve la probabilidad con que el mismo afirma que
    /// se trata de un incidente.
    fn process_response(&self, res_text: &str) -> Result<f64, Box<dyn Error>> {
        let res_json: serde_json::Value = serde_json::from_str(res_text)?;

        let incident_probability_option  = res_json["predictions"]
            .as_array()
            .and_then(|predictions| {
                predictions.iter().find_map(|prediction| {
                    let tag = self.properties.get_inc_tag();
                    if prediction["tagName"].as_str() == Some(tag) {
                        prediction["probability"].as_f64()
                    } else {
                        None
                    }
                })
            });
        
        if let Some(incident_probability) = incident_probability_option {
            Ok(incident_probability)
        } else {
            println!("Response raw recibida: {:?}.", res_json);
            self.logger.log(format!("Response raw recibida: {:?}.", res_json));
            Err(Box::new(std::io::Error::new(ErrorKind::Other, "Error al obtener la incident_probability.")))
        }
    }

    /// Recibe el image_path de la imagen en la que se detectó un incidente, crea el Incident y lo envía internamente para
    /// ser publicado por MQTT.
    fn process_incident(&mut self, cam_id: u8) -> Result<(), Box<dyn Error>>{
        // obtenemos la posición
        let incident_position: (f64, f64) = self.get_incident_position(cam_id)?;
        // creamos el incidente
        let inc_id = self.get_next_incident_id()?;
        let incident = Incident::new(inc_id, incident_position, IncidentSource::Automated);
        
        println!("Detector: Incidente creado! {:?}", incident);
        self.logger.log(format!("Detector: Incidente creado! {:?}", incident));
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

    fn get_next_incident_id(&mut self) -> Result<u8, std::io::Error> {
        if let Ok(mut last) = self.last_incident_id.lock() {
            *last += 1;
            return Ok(*last);

        }
        Err(std::io::Error::new(ErrorKind::Other, "Detector: Error al tomar el lock"))
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

