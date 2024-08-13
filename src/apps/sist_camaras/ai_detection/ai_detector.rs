use rand::{thread_rng, Rng};
use reqwest::{
    blocking::Client,
    header::{HeaderMap, CONTENT_TYPE},
};
use std::{
    error::Error,
    io::ErrorKind,
    sync::{mpsc, Arc, Mutex},
};

use crate::{
    apps::{
        incident_data::{incident::Incident, incident_source::IncidentSource},
        sist_camaras::{
            ai_detection::{api_credentials::ApiCredentials, properties::DetectorProperties},
            types::shareable_cameras_type::ShCamerasType,
        },
    },
    logging::string_logger::StringLogger,
};

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
    pub fn new(
        cameras: ShCamerasType,
        tx: mpsc::Sender<Incident>,
        properties: DetectorProperties,
        logger: StringLogger,
    ) -> Self {
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

        println!("DEBUG: Image size: {}", image.len()); // debug

        // Se envía la imagen al proveedor
        let res = client
            .post(api_credentials.get_endpoint())
            .headers(headers)
            .body(image)
            .send()?;

        println!("DEBUG: res.status: {}", res.status()); // debug

        let res_text = res.text()?;
        let incident_probability = self.process_response(&res_text)?;

        println!("Detector: Probability: {:?}", incident_probability);
        self.logger
            .log(format!("Detector: Probability: {:?}", incident_probability));
        if incident_probability > self.properties.get_inc_threshold() {
            self.process_incident(cam_id)?;
        }

        Ok(())
    }

    /// Interpreta el res_text recibido como json y devuelve la probabilidad con que el mismo afirma que
    /// se trata de un incidente.
    fn process_response(&self, res_text: &str) -> Result<f64, Box<dyn Error>> {
        let res_json: serde_json::Value = serde_json::from_str(res_text)?;

        // Analizamos primero si la respuesta fue un error
        self.process_error_response(&res_json)?;

        // Si no lo fue, buscamos la probability
        let incident_probability_option =
            res_json["predictions"].as_array().and_then(|predictions| {
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
            println!("Response raw recibida: {}.", res_json);
            self.logger
                .log(format!("Response raw recibida: {}.", res_json));
            Err(Box::new(std::io::Error::new(
                ErrorKind::Other,
                "Error al obtener la incident_probability.",
            )))
        }
    }

    /// Analiza si la respuesta de la api informa de un error.
    /// Si es un error, lo devuelve. Caso contrario devuelve Ok.
    fn process_error_response(
        &self,
        res_json: &serde_json::Value,
    ) -> Result<(), Box<std::io::Error>> {
        if let Some((error_code, error_msg)) =
            self.get_error_code_and_msg_from_error_response(res_json)
        {
            // Devolver el error
            let displayable_error = format!(
                "Response API es error: code {}, message: {}.",
                error_code, error_msg
            );
            return Err(Box::new(std::io::Error::new(
                ErrorKind::Other,
                displayable_error,
            )));
        }

        Ok(())
    }

    /// Parsea la response en busca de `code` y `message` de una respuesta de la api de tipo error.
    /// Si la respuesta de la api informa que hubo un error, devuelve Some de `code` y `message`.
    /// Caso contrario devuelve None.
    fn get_error_code_and_msg_from_error_response(
        &self,
        res_json: &serde_json::Value,
    ) -> Option<(String, String)> {
        if let Some(error) = res_json.get("error") {
            // Errores con formato { "error": {"code": ..., "message": ... } }
            let code = error.get("code").and_then(|c| c.as_str());
            let message = error.get("message").and_then(|m| m.as_str());

            if let (Some(cod), Some(msg)) = (code, message) {
                return Some((cod.to_string(), msg.to_string()));
            }
        } else if let Some(code) = res_json.get("code").and_then(|c| c.as_str()) {
            // Errores con formato { "code": "...", "message": "..." }
            if let Some(message) = res_json.get("message").and_then(|c| c.as_str()) {
                return Some((code.to_string(), message.to_string()));
            }
        }
        None
    }

    /// Recibe el image_path de la imagen en la que se detectó un incidente, crea el Incident y lo envía internamente para
    /// ser publicado por MQTT.
    fn process_incident(&mut self, cam_id: u8) -> Result<(), Box<dyn Error>> {
        // obtenemos la posición
        let incident_position: (f64, f64) = self.get_incident_position(cam_id)?;
        // creamos el incidente
        let inc_id = self.get_next_incident_id()?;
        let incident = Incident::new(inc_id, incident_position, IncidentSource::Automated);

        println!("Detector: Incidente creado! {:?}", incident);
        self.logger
            .log(format!("Detector: Incidente creado! {:?}", incident));
        // se envía el inc para ser publicado
        self.tx.send(incident)?;
        Ok(())
    }

    /// Genera una ubicación de incidente aleatoria
    /// dentro del rango de la camara que detectó el incidente.
    fn get_incident_position(&self, camera_id: u8) -> Result<(f64, f64), std::io::Error> {
        if let Ok(cameras) = self.cameras.lock() {
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

        Err(std::io::Error::new(
            ErrorKind::Other,
            "Error al obtener la camera del hashmap en get_incident_position.",
        ))
    }

    /// Obtiene el siguiente incident id disponible para utilizar.
    /// Al ser éste un programa multihilo, es necesario que el manejo de esta variable sea atómico
    /// para no tener problemas de concurrencia que lleven a ids duplicados.
    fn get_next_incident_id(&mut self) -> Result<u8, std::io::Error> {
        if let Ok(mut last) = self.last_incident_id.lock() {
            *last += 1;
            return Ok(*last);
        }
        Err(std::io::Error::new(
            ErrorKind::Other,
            "Detector: Error al tomar el lock",
        ))
    }
}

fn create_client_and_headers(
    api_credentials: &ApiCredentials,
) -> Result<(Client, HeaderMap), Box<dyn Error>> {
    let client = Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        "Prediction-Key",
        api_credentials.get_prediction_key().parse()?,
    );
    headers.insert(CONTENT_TYPE, "application/octet-stream".parse()?);
    Ok((client, headers))
}
