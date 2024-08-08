use notify::event::EventKind;
use notify::{RecursiveMode, Watcher};
use rayon::ThreadPoolBuilder;
use std::error::Error;
use std::sync::mpsc::Receiver;
use std::{fs, thread};
use std::io::ErrorKind;
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex};

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
    inc_tx: mpsc::Sender<Incident>,
    exit_requested: Arc<Mutex<bool>>,
    logger: StringLogger,
}

impl AIDetectorManager {
    pub fn new(cameras: ShCamerasType, inc_tx: mpsc::Sender<Incident>, exit_rx: mpsc::Receiver<()>, logger: StringLogger) -> Self {
        
        let er = Arc::new(Mutex::new(false));

        let detector_manager = Self {
            cameras,
            inc_tx,
            exit_requested: er.clone(),
            logger,
        };

        let handle = thread::spawn(move || {
            modify_if_exit_requested(er, exit_rx);
        });

        handle.join().unwrap(); // aux, ahora lo cambio

        detector_manager.run();

        detector_manager
    }

    /// Crea y monitorea los subdirectorios correspondientes a las cámaras, cuando una imagen se crea en alguno de ellos,
    /// se lanza el procedimiento para analizar mediante proveedor de servicio de inteligencia artificial si la misma
    /// contiene o no un incidente, y se lo envía internamente a Sistema Cámaras para que sea publicado por MQTT.
    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        




        let properties = DetectorProperties::new(PROPERTIES_FILE)?;
        // Crea, si no existían, el dir base y los subdirectorios, y los monitorea
        let path = Path::new(properties.get_base_dir());
        self.create_dirs_tree(path)?;
    
        let (tx_fs, rx_fs) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx_fs)?;
        watcher.watch(path, RecursiveMode::Recursive)?;
        println!("Detector: Monitoreando subdirs.");
        self.logger.log("Detector: Monitoreando subdirs".to_string());
    
        // Se inicializa el detector
        let logger_ai = self.logger.clone_ref();
        let ai_detector = AutomaticIncidentDetector::new(self.cameras.clone(), self.inc_tx.clone(), properties, logger_ai);
    
        // Crear un pool de threads con el número de threads deseado
        let pool = ThreadPoolBuilder::new().num_threads(6).build()?;

        if self.exit_requested() {
            println!("   hilo detector pre for: por hacer return");
            return Ok(());
        }
        for event_res in rx_fs {
            // Sale, si lo solicitaron desde abm
            if self.exit_requested() {
                println!("   hilo detector pre for: por hacer return");
                break;
            }

            // Procesa el evento, interesa el Create, que es cuando se crea una imagen en algún subdirectorio
            let event = event_res?;
            if let EventKind::Create(_) = event.kind {
                self.logger.log("Detector: event ok: create".to_string());
                if let Some(path) = event.paths.first() {
                    if let Err(e) = self.launch_detection_for_image(&ai_detector, &pool, path){
                        println!("Detector: Error al procesar la imagen: {:?}, {:?}", path, e);
                        self.logger.log(format!("Detector: Error al procesar la imagen: {:?}, {:?}", path, e));
                    }
                }
                
            }
        }
        
        Ok(())
    }
    
    /// Crea, si no existía, la estructura de directorios necesaria para las imágenes de las cámaras.
    fn create_dirs_tree(&self, base_dir: &Path) -> Result<(), std::io::Error> {    
        self.create_basedir(base_dir)?;
        self.create_subdirs(base_dir)?;
        Ok(())
    }
    
    /// Crea el `base_dir` que contendrá a los subdirectorios de las cámaras, si no existía.
    fn create_basedir(&self, base_dir: &Path) -> Result<(), std::io::Error> {    
        // Si ya existe, lo borra y a todo su contenido, y
        if base_dir.exists() {
            fs::remove_dir_all(base_dir)?;
        }
        // lo crea
        fs::create_dir(base_dir)?;
    
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

    /// Envía el pedido a la threadpool para detectar incidente en la imagen.
    fn launch_detection_for_image(&self, ai_detector: &AutomaticIncidentDetector, pool: &rayon::ThreadPool, path: &Path) -> Result<(), Box<dyn Error>>{
            if path.is_file() {
                let image_path = path.to_owned();
                // Validar la extensión del archivo
                is_valid_extension(&image_path)?;
                
                // Lanza un hilo por cada imagen a procesar []
                let mut aidetector = ai_detector.clone_refs();
                let logger_c = self.logger.clone_ref();
                pool.spawn(move || {
                    if let Err(e) = read_and_process_image(&mut aidetector, &image_path){
                        println!("Detector: Error en read_and_process_image: {:?}.", e);
                        logger_c.log(format!("Detector: Error en read_and_process_image: {:?}.", e));
                    }
                });

            }
        Ok(())        
    }
    
    

    /// Devuelve si se solicitó salir.
    fn exit_requested(&self) -> bool {
        if let Ok(var) = self.exit_requested.lock(){
            return *var 
        }
        false
    }
    
}

/// Si recibe por el `rx` que se solicitó salir, lo deja asentado en la variable compartida `exit_requested`,
/// para que en la próxima vuelta del for el detector manager se entere y finalice la ejecución.
fn modify_if_exit_requested(exit_requested: Arc<Mutex<bool>>, rx: Receiver<()>) {
    if let Ok(_) = rx.recv() {
        if let Ok(mut var) = exit_requested.lock(){
            *var = true;
        }
    }
}

/// Lee la imagen del archivo path proporcionado y llama a procesarla.
fn read_and_process_image(aidetector: &mut AutomaticIncidentDetector, image_path: &Path) -> Result<(), Box<dyn Error>> {
    let img = read_image(image_path)?;
    if let Some(cam_id) = extract_camera_id(image_path) {
        aidetector.process_image(img, cam_id)?;
    };
    Ok(())
}

/// Checkea si la extensión de la imagen es válida.
fn is_valid_extension(image_path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(extension) = image_path.extension() {
        // Si es jpg o jpeg, procesar
        if extension == "jpg" || extension == "jpeg" {
            return Ok(());
        }
    }
    Err(Box::new(std::io::Error::new(
        ErrorKind::Other,
        "Extensión inválida.",
    )))
}

/// Lee la imagen del `image_path`.
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