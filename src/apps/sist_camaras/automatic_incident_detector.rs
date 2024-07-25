use std::sync::mpsc;

use crate::apps::incident_data::incident::Incident;

use super::shareable_cameras_type::ShCamerasType;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use std::error::Error;

/// Módulo de detección automática de incidentes de Sistema Cámaras.
/// Al detectar un incidente en una imagen tomada por una Cámara,
/// crea el incidente y lo envía por el tx, para que el sistema cámaras pueda publicarlo por mqtt.
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
        let _response_json = query_ia_provider("./accidente.jpeg").unwrap(); //(aux: dsp saco el unwrap)
        // Usar algún crate que me parsee el json a un objeto más copado.
        // para que podamos hacerle tags.contain("blbala")

        



    }
}

fn query_ia_provider(img_name: &str) -> Result<String, Box<dyn Error>> {
    // Crear el cliente HTTP bloqueante
    let client = Client::builder().build()?;

    // Configurar las cabeceras
    let mut headers = HeaderMap::new();
    headers.insert("Ocp-Apim-Subscription-Key", HeaderValue::from_static("6287679ec0674fea8e5a27d8a8455ae9"));
    headers.insert("Content-Type", HeaderValue::from_static("application/octet-stream")); // esto es para url

    // Definir los datos JSON
    /*let data = r#"{
        "url": "https://upload.wikimedia.org/wikipedia/commons/8/8f/Fire_inside_an_abandoned_convent_in_Massueville%2C_Quebec%2C_Canada.jpg"
    }"#; // una url de imagen
    let json: Value = serde_json::from_str(data)?;*/

    let bytes = std::fs::read(img_name)?;

    // Construir la solicitud
    let request = client
        .request(reqwest::Method::POST, "https://computervisiontaller1.cognitiveservices.azure.com/vision/v3.2/tag")
        .headers(headers)
        //.json(&json); // para url
        .body(bytes);

    // Enviar la solicitud y obtener la respuesta
    let response = request.send()?;
    let body = response.text()?;

    Ok(body)
}