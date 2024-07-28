

use notify::event::EventKind;
use notify::{RecursiveMode, Watcher};
use rand::{thread_rng, Rng};
use rayon::ThreadPoolBuilder;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, CONTENT_TYPE};
use std::error::Error;
use std::fs;
use std::io::ErrorKind;
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

    pub fn clone_ref(&self) -> Self {
        Self {
            cameras: self.cameras.clone(),
            tx: self.tx.clone(),
            last_incident_id: self.last_incident_id,
        }
    }


    /// Crea subdirectorios de `base_dir`, uno por cada cámara, de nombre "camera_i"
    /// donde `i` es el id de dicha cámara.
    fn create_subdirs(&self, base_dir: &Path) -> Result<(), std::io::Error> {    
        if let Ok(cameras) = self.cameras.lock() {
            for cam in cameras.values() {
                if cam.is_not_deleted() {
                    // (para todas va a dar true, porque Sistema Camaras se está iniciando, pero así es más genérico)
                    let cam_id = cam.get_id();
                    self.create_subdir(base_dir, cam_id)?;
                }
            }
        }
    
        Ok(())
    }

    /// Crea un subdirectorio de `base_dir` de nombre "camera_i" donde `i` es el u8 recibido.
    fn create_subdir(&self, base_dir: &Path, i: u8) -> Result<(), std::io::Error> {    
        // Concatena el nombre del subdir a crear, al dir base
        let subdir = format!("camera_{}", i);
        let new_dir_path = base_dir.join(subdir);

        // Si no existe, lo crea
        if !new_dir_path.exists() {
            fs::create_dir(&new_dir_path)?;
        }
    
        Ok(())
    }

    /// Crea el `base_dir` que contendrá a los subdirectorios de las cámaras, si no existía.
    fn create_basedir(&self, base_dir: &Path) -> Result<(), std::io::Error> {    
        // Si no existe, lo crea
        if !base_dir.exists() {
            fs::create_dir(&base_dir)?;
        }
    
        Ok(())
    }

    /// Crea y monitorea los subdirectorios correspondientes a las cámaras, cuando una imagen se crea en alguno de ellos,
    /// se lanza el procedimiento para analizar mediante proveedor de servicio de inteligencia artificial si la misma
    /// contiene o no un incidente, y se lo envía internamente a Sistema Cámaras para que sea publicado por MQTT.
    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        // Crea, si no existían, el dir base y los subdirectorios, y los monitorea
        let path = Path::new("./src/apps/sist_camaras/azure_model/image_detection");
        self.create_basedir(path)?;
        self.create_subdirs(path)?;
        let (tx_fs, rx_fs) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx_fs)?;
        watcher.watch(path, RecursiveMode::Recursive)?;

        // Crear un pool de threads con el número de threads deseado
        let pool = ThreadPoolBuilder::new().num_threads(6).build()?;

        for event_res in rx_fs {
            let mut self_clone = self.clone_ref();            
            let event = event_res?;

            // Interesa el evento Create, que es cuando se crea una imagen en algún subdirectorio
            if let EventKind::Create(_) = event.kind {
                println!("event ok: create");
                if let Some(path) = event.paths.first() {
                    if path.is_file() {
                        let image_path = path.clone();
                        // Lanza un hilo por cada imagen a procesar []
                        pool.spawn(move || {
                            if process_image(image_path, &mut self_clone).is_err() {
                                println!("Error en process_image.");
                            };
                        });
                    }
                }
            }
        }

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

/// Recibe el path de la imagen que se está procesando, obtiene el id
/// de la cámara que capturó dicha imagen. Es decir la parte que sigue a "camera_"
/// de su carpeta padre.
fn extract_camera_id(path: &Path) -> Option<u8> {
    // Obtiene el nombre del directorio padre
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|file_name| file_name.to_str())
        .and_then(|name| {
            // El nombre del directorio tiene el formato "camera_u8"
            let prefix = "camera_";
            if name.starts_with(prefix) {
                name[prefix.len()..].parse().ok()
            } else {
                None
            }
        })
}


/// Lee la imagen de `image_path`, se la envía al proveedor de ia y analiza su respuesta para concluir si
/// la imagen contiene o no un incidente. En caso afirmativo, se procesa al incidente.
fn process_image(image_path: PathBuf, self_clone: &mut AutomaticIncidentDetector) -> Result<(), Box<dyn Error>> {
    let buffer = read_image(&image_path)?;
    let api_credentials = ApiCredentials::new();
    
    let (client, headers) = create_client_and_headers(&api_credentials)?;

    // Se envía la imagen al proveedor
    let res = client
        .post(api_credentials.get_endpoint())
        .headers(headers)
        .body(buffer)
        .send()?;

    let res_text = res.text()?;
    let incident_probability = process_response(&res_text)?;

    println!("Probability: {:?}", incident_probability);
    if incident_probability > 0.7 {
        process_incident(image_path, self_clone)?;
    }

    Ok(())
}

/// Recibe el image_path de la imagen en la que se detectó un incidente, crea el Incident y lo envía internamente para
/// ser publicado por MQTT.
fn process_incident(image_path: PathBuf, self_clone: &mut AutomaticIncidentDetector) -> Result<(), Box<dyn Error>>{
    println!("image_path: {:?}", image_path);
    if let Some(camera_id) = extract_camera_id(&image_path) {
        // obtenemos la posición
        println!("camera_id: {}", camera_id);
        let incident_position: (f64, f64) = self_clone.get_incident_position(camera_id)?;
        // creamos el incidente
        let inc_id = self_clone.get_next_incident_id();
        let incident = Incident::new(inc_id, incident_position, IncidentSource::Automated);
        println!("Incidente creado! {:?}", incident);
        // se envía el inc para ser publicado
        self_clone.tx.send(incident)?;
        Ok(())
    } else {
        Err(Box::new(std::io::Error::new(ErrorKind::Other, "Error al extraer el cam_id del directorio.")))
    }
}

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

