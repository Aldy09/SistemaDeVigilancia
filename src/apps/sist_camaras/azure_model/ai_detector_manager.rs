use notify::event::EventKind;
use notify::{RecursiveMode, Watcher};
use rayon::ThreadPoolBuilder;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::mpsc;

use crate::apps::sist_camaras::azure_model::automatic_incident_detector::AutomaticIncidentDetector;
use crate::apps::sist_camaras::shareable_cameras_type::ShCamerasType;
use crate::apps::incident_data::incident::Incident;

#[derive(Debug)]
/// Se encarga de inicializar todo lo relacionado a directorios, monitorearlos, y threads,
/// y finalmente llama al Automatic Incident Detector cuando se crea una imagen en algún subdirectorio
/// de alguna cámara.
pub struct AIDetectorManager {
    cameras: ShCamerasType,
    tx: mpsc::Sender<Incident>,
}

impl AIDetectorManager {
    pub fn new(cameras: ShCamerasType, tx: mpsc::Sender<Incident>) -> Self {
        Self {
            cameras,
            tx,
        }
    }

    /// Crea y monitorea los subdirectorios correspondientes a las cámaras, cuando una imagen se crea en alguno de ellos,
    /// se lanza el procedimiento para analizar mediante proveedor de servicio de inteligencia artificial si la misma
    /// contiene o no un incidente, y se lo envía internamente a Sistema Cámaras para que sea publicado por MQTT.
    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        // Crea, si no existían, el dir base y los subdirectorios, y los monitorea
        let path = Path::new("./src/apps/sist_camaras/azure_model/image_detection");
        self.create_dirs_tree(path)?;

        let (tx_fs, rx_fs) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx_fs)?;
        watcher.watch(path, RecursiveMode::Recursive)?;

        // Se inicializa el detector
        let ai_detector = AutomaticIncidentDetector::new(self.cameras.clone(), self.tx.clone());

        // Crear un pool de threads con el número de threads deseado
        let pool = ThreadPoolBuilder::new().num_threads(6).build()?;

        for event_res in rx_fs {
            let event = event_res?;

            // Interesa el evento Create, que es cuando se crea una imagen en algún subdirectorio
            if let EventKind::Create(_) = event.kind {
                println!("event ok: create");
                if let Some(path) = event.paths.first() {
                    if path.is_file() {
                        let image_path = path.clone();
                        // Lanza un hilo por cada imagen a procesar []
                        let mut aidetector = ai_detector.clone_refs();
                        pool.spawn(move || {

                            match read_image(&image_path) {
                                Ok(image) => {
                                    if let Some(cam_id) = extract_camera_id(&image_path){
                                        if aidetector.process_image(image, cam_id).is_err() {
                                            println!("Error en process_image.");
                                        }
                                    }
                                },
                                Err(e) => println!("Error al leer la imagen: {:?}, {:?}", image_path, e),
                            };
                            
                        });
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
                name[prefix.len()..].parse().ok()
            } else {
                None
            }
        })
}