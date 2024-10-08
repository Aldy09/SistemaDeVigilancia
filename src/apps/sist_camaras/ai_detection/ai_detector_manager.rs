use notify::{event::EventKind, RecursiveMode, Watcher};
use rayon::ThreadPoolBuilder;
use std::{
    error::Error,
    ffi::OsStr,
    fs,
    io::{Error as ioError, ErrorKind},
    path::Path,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};

use crate::{
    apps::{
        incident_data::incident::Incident,
        sist_camaras::{
            ai_detection::{
                ai_detector::AutomaticIncidentDetector, properties::DetectorProperties,
            },
            types::shareable_cameras_type::ShCamerasType,
        },
    },
    logging::string_logger::StringLogger,
};

const PROPERTIES_FILE: &str = "./src/apps/sist_camaras/ai_detection/properties.txt";

#[derive(Debug)]
/// Se encarga de inicializar todo lo relacionado a directorios, monitorearlos, y threads,
/// y finalmente llama al Automatic Incident Detector cuando se crea una imagen en algún subdirectorio
/// de alguna cámara.
pub struct AIDetectorManager {
    cameras: ShCamerasType,
    inc_tx: Sender<Incident>,
    exit_requested: Arc<Mutex<bool>>,
    properties: DetectorProperties,
    logger: StringLogger,
}

impl AIDetectorManager {
    /// Crea y ejecuta lo necesario para la detección de incidentes de manera automática haciendo uso de inteligencia artificial,
    pub fn run(
        cameras: ShCamerasType,
        inc_tx: mpsc::Sender<Incident>,
        exit_rx: mpsc::Receiver<()>,
        logger: StringLogger,
    ) -> Result<Self, ioError> {
        let properties = DetectorProperties::new(PROPERTIES_FILE)?;

        let er = Arc::new(Mutex::new(false));
        let detector_manager = Self {
            cameras,
            inc_tx,
            exit_requested: er.clone(),
            properties,
            logger,
        };

        // Lanza hilo que pondrá en true la `er` si se solicita salir desde abm
        let handle = thread::spawn(move || {
            modify_if_exit_requested(er, exit_rx);
        });

        // Se ejecuta el detector
        if let Err(e) = detector_manager.run_internal() {
            detector_manager
                .logger
                .log(format!("Error en ejecución de detector: {:?}.", e));
        }

        // Espera al hilo lanzado
        if let Err(e) = handle.join() {
            detector_manager.logger.log(format!(
                "Error al joinear hilo de exit de detector: {:?}.",
                e
            ));
        }

        Ok(detector_manager)
    }

    /// Crea y monitorea los subdirectorios correspondientes a las cámaras, cuando una imagen se crea en alguno de ellos,
    /// se lanza el procedimiento para analizar mediante proveedor de servicio de inteligencia artificial si la misma
    /// contiene o no un incidente, y se lo envía internamente a Sistema Cámaras para que sea publicado por MQTT.
    fn run_internal(&self) -> Result<(), Box<dyn Error>> {
        // Crea, si no existían, el dir base y los subdirectorios, y los monitorea
        let path = Path::new(self.properties.get_base_dir());
        self.create_dirs_tree(path)?;
        // Comienza a monitorear los directorios
        let (tx_fs, rx_fs) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx_fs.clone())?;
        watcher.watch(path, RecursiveMode::Recursive)?;
        println!("Detector: Monitoreando subdirs.");
        self.logger
            .log("Detector: Monitoreando subdirs".to_string());

        // Se inicializa el detector
        let logger_ai = self.logger.clone_ref();
        let ai_detector = AutomaticIncidentDetector::new(
            self.cameras.clone(),
            self.inc_tx.clone(),
            self.properties.clone(),
            logger_ai,
        );

        // Crear un pool de threads con el número de threads deseado
        let pool = ThreadPoolBuilder::new().num_threads(6).build()?;

        for event_res in rx_fs {
            // Sale, si lo solicitaron desde abm
            if self.exit_requested() {
                break;
            }

            // Procesa el evento, interesa el Create, que es cuando se crea una imagen en algún subdirectorio
            let event = event_res?;
            if let EventKind::Create(_) = event.kind {
                self.logger.log("Detector: event ok: create".to_string());
                if let Some(path) = event.paths.first() {
                    if let Err(e) = self.launch_detection_for_image(&ai_detector, &pool, path) {
                        println!("Detector: Error al procesar la imagen: {:?}, {:?}", path, e);
                        self.logger.log(format!(
                            "Detector: Error al procesar la imagen: {:?}, {:?}",
                            path, e
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Crea, si no existía, la estructura de directorios necesaria para las imágenes de las cámaras.
    fn create_dirs_tree(&self, base_dir: &Path) -> Result<(), ioError> {
        self.create_basedir(base_dir)?;
        self.create_subdirs(base_dir)?;
        Ok(())
    }

    /// Crea el `base_dir` que contendrá a los subdirectorios de las cámaras, si no existía.
    fn create_basedir(&self, base_dir: &Path) -> Result<(), ioError> {
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
    fn create_subdirs(&self, base_dir: &Path) -> Result<(), ioError> {
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
    fn create_subdir(&self, base_dir: &Path, i: u8) -> Result<(), ioError> {
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
    fn launch_detection_for_image(
        &self,
        ai_detector: &AutomaticIncidentDetector,
        pool: &rayon::ThreadPool,
        path: &Path,
    ) -> Result<(), Box<dyn Error>> {
        if path.is_file() {
            let image_path = path.to_owned();
            // Validar la extensión del archivo
            self.is_valid_extension(&image_path)?;

            // Ejecuta el procesamiento de la imagen en un hilo de la threadpool
            let mut aidetector = ai_detector.clone_refs();
            let logger_c = self.logger.clone_ref();
            pool.spawn(move || {
                if let Err(e) = read_and_process_image(&mut aidetector, &image_path) {
                    println!("Detector: Error en read_and_process_image: {:?}.", e);
                    logger_c.log(format!(
                        "Detector: Error en read_and_process_image: {:?}.",
                        e
                    ));
                }
            });
        }
        Ok(())
    }

    /// Checkea si la extensión de la imagen es válida.
    fn is_valid_extension(&self, image_path: &Path) -> Result<(), Box<dyn Error>> {
        if let Some(img_extension) = image_path.extension().and_then(OsStr::to_str) {
            // Si es una extensión válida (ej jpg o jpeg), procesar
            let valid_extensions = self.properties.get_img_valid_extensions();
            if valid_extensions.contains(&img_extension) {
                return Ok(());
            }
        }
        Err(Box::new(ioError::new(
            ErrorKind::Other,
            "Extensión inválida.",
        )))
    }

    /// Devuelve si se solicitó salir.
    fn exit_requested(&self) -> bool {
        if let Ok(var) = self.exit_requested.lock() {
            return *var;
        }
        false
    }
}

/// Si recibe por el `rx` que se solicitó salir, lo deja asentado en la variable compartida `exit_requested`,
/// para que en la próxima vuelta del for el detector manager se entere y finalice la ejecución.
fn modify_if_exit_requested(exit_requested: Arc<Mutex<bool>>, rx: Receiver<()>) {
    if rx.recv().is_ok() {
        if let Ok(mut var) = exit_requested.lock() {
            *var = true;
        }
    }
}

/// Lee la imagen del archivo path proporcionado y llama a procesarla.
fn read_and_process_image(
    aidetector: &mut AutomaticIncidentDetector,
    image_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let img = read_image(image_path)?;
    if let Some(cam_id) = extract_camera_id(image_path) {
        aidetector.process_image(img, cam_id)?;
    };
    Ok(())
}

/// Lee la imagen del `image_path`.
fn read_image(image_path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut file = std::fs::File::open(image_path)?;
    let mut image_buffer = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut image_buffer)?;

    println!("DEBUG: Image size en read_image: {}", image_buffer.len()); // debug
    if image_buffer.is_empty() {
        return Err(Box::new(ioError::new(
            ErrorKind::Other,
            "La imagen tiene tamaño 0.",
        )));
    }
    Ok(image_buffer)
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
                    return cam_id.parse().ok();
                }
            }
            None
        })
}
