

use rayon::{ThreadPool, ThreadPoolBuilder};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use notify::{self, Event, EventKind, RecursiveMode, Watcher};
use std::sync::mpsc::{Receiver, Sender};



use crate::apps::sist_camaras::ai_detection::ai_detector::AutomaticIncidentDetector;
use crate::apps::sist_camaras::types::shareable_cameras_type::ShCamerasType;
use crate::apps::incident_data::incident::Incident;
use crate::logging::string_logger::StringLogger;

use super::properties::DetectorProperties;

const PROPERTIES_FILE: &str = "./src/apps/sist_camaras/ai_detection/properties.txt";

#[derive(Debug)]
/// Se encarga de inicializar todo lo relacionado a directorios, monitorearlos, y threads,
/// y finalmente llama al Automatic Incident Detector cuando se crea una imagen en algún subdirectorio
/// de alguna cámara.
pub struct AIDetectorManager {
    cameras: ShCamerasType,
    tx: mpsc::Sender<Incident>,
    logger: StringLogger,
}

impl AIDetectorManager {
    pub fn new(cameras: ShCamerasType, tx: mpsc::Sender<Incident>, logger: StringLogger) -> Self {
        Self {
            cameras,
            tx,
            logger,
        }
    }

    /// Crea y monitorea los subdirectorios correspondientes a las cámaras, cuando una imagen se crea en alguno de ellos,
    /// se lanza el procedimiento para analizar mediante proveedor de servicio de inteligencia artificial si la misma
    /// contiene o no un incidente, y se lo envía internamente a Sistema Cámaras para que sea publicado por MQTT.
    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        let properties = self.initialize_detector_properties()?;
        let path = self.create_and_watch_directories(&properties)?;
        let (tx_fs, rx_fs) = mpsc::channel();
        self.start_watching_directories(&path, tx_fs)?;
        let ai_detector = self.initialize_ai_detector(&properties);
        let pool = self.create_thread_pool()?;
        self.process_filesystem_events(rx_fs, &ai_detector, &pool)?;
        Ok(())
    }

    fn initialize_detector_properties(&self) -> Result<DetectorProperties, Box<dyn Error>> {
        let properties = DetectorProperties::new(PROPERTIES_FILE)?;
        Ok(properties)
    }

    fn create_and_watch_directories(&self, properties: &DetectorProperties) -> Result<PathBuf, Box<dyn Error>> {
        let path = PathBuf::from(properties.get_base_dir());
        self.create_dirs_tree(&path)?;
        Ok(path)
    }

    fn initialize_ai_detector(&self, properties: &DetectorProperties) -> AutomaticIncidentDetector {
        let logger_ai = self.logger.clone_ref();
        AutomaticIncidentDetector::new(self.cameras.clone(), self.tx.clone(), properties.clone(), logger_ai)
    }

    fn create_thread_pool(&self) -> Result<ThreadPool, Box<dyn Error>> {
        let pool = ThreadPoolBuilder::new().num_threads(6).build()?;
        Ok(pool)
    }

    fn start_watching_directories(&self, path: &Path, tx_fs: Sender<notify::Result<Event>>) -> Result<(), Box<dyn Error>> {
        let mut watcher = notify::recommended_watcher(tx_fs)?;
        watcher.watch(path, RecursiveMode::Recursive)?;
        println!("Detector: Monitoreando subdirs.");
        self.logger.log("Detector: Monitoreando subdirs".to_string());
        Ok(())
    }

    fn process_filesystem_events(&self, rx_fs: Receiver<notify::Result<Event>>, ai_detector: &AutomaticIncidentDetector, pool: &ThreadPool) -> Result<(), Box<dyn Error>> {
        for event_res in rx_fs {
            let event = event_res?;
            if let EventKind::Create(_) = event.kind {
                self.logger.log("Detector: event ok: create".to_string());
                if let Some(path) = event.paths.first() {
                    if path.is_file() && self.is_jpeg_or_jpg(path) {
                        self.process_image_file(path, ai_detector, pool)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn process_image_file(&self, image_path: &Path, ai_detector: &AutomaticIncidentDetector, pool: &ThreadPool) -> Result<(), Box<dyn Error>> {
        let mut aidetector = ai_detector.clone_refs();
        let image_path = image_path.to_path_buf();
        pool.spawn(move || {
            match read_image(&image_path) {
                Ok(image) => {
                    if let Some(cam_id) = extract_camera_id(&image_path) {
                        if aidetector.process_image(image, cam_id).is_err() {
                            println!("Detector: Error en process_image.");
                        }
                    }
                },
                Err(e) => println!("Detector: Error al leer la imagen: {:?}, {:?}", image_path, e),
            };
        });
        Ok(())
    }

    fn is_jpeg_or_jpg(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            extension == "jpg" || extension == "jpeg"
        } else {
            false
        }
    }
    
    /// Crea, si no existía, la estructura de directorios necesaria para las imágenes de las cámaras.
    fn create_dirs_tree(&self, base_dir: &Path) -> Result<(), std::io::Error> {    
        self.create_basedir(base_dir)?;
        self.create_subdirs(base_dir)?;
        Ok(())
    }
    
    /// Crea el `base_dir` que contendrá a los subdirectorios de las cámaras, si no existía.
    fn create_basedir(&self, base_dir: &Path) -> Result<(), std::io::Error> {    
        // Si no existe, lo crea
        if !base_dir.exists() {
            fs::create_dir(base_dir)?;
        }
    
        Ok(())
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


    

}

fn read_image(image_path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut file = std::fs::File::open(image_path)?;
    let mut buffer = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buffer)?;
    Ok(buffer)
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
                if let Some(cam_id) = name.strip_prefix(prefix) {
                    return cam_id.parse().ok()
                }
            }
                None
            
        })
}