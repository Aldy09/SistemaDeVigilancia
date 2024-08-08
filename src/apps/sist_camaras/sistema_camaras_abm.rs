use std::{
    collections::HashMap, io::{stdin, stdout, Error, Write}, sync::{
        mpsc::Sender,
        Arc, Mutex,
    }
};

use crate::logging::string_logger::StringLogger;

use super::camera::Camera;

pub struct ABMCameras {
    cameras: Arc<Mutex<HashMap<u8, Camera>>>,
    camera_tx: Sender<Vec<u8>>,
    exit_tx: Sender<bool>,
    logger: StringLogger,
}

impl ABMCameras {
    /// Crea un struct `ABMCameras`.
    pub fn new(
        cameras: Arc<Mutex<HashMap<u8, Camera>>>,
        camera_tx: Sender<Vec<u8>>,
        exit_tx: Sender<bool>,
        logger: StringLogger,
    ) -> Self {
        ABMCameras {
            cameras,
            camera_tx,
            exit_tx,
            logger,
        }
    }

    /// Pone en funcionamiento el menú del abm para cámaras.
    /// Como cameras es un arc, quien haya llamado a esta función podrá ver reflejados los cambios.
    pub fn run(&mut self) {
        // Publica cámaras al inicio
        self.send_cameras_from_file_to_publish();
        // Ejecuta el menú
        loop {
            self.print_menu_abm();
            let input = self.get_input_abm(None);

            match &*input {
                "1" => {
                    self.create_camera_abm();
                }
                "2" => self.show_cameras_abm(),
                "3" => self.delete_camera_abm(),
                "4" => {
                    self.exit_program_abm();
                    break;
                }
                _ => {
                    println!("Opción no válida. Intente nuevamente.\n");
                }
            }
        }
    }

    /// Muestra por pantalla el menú.
    fn print_menu_abm(&self) {
        println!(
            "      MENÚ
        1. Agregar cámara
        2. Mostrar cámaras
        3. Eliminar cámara
        4. Salir
        Ingrese una opción:"
        );
    }

    /// Opción Crear cámara, del abm. Crea una cámara con el input proporcionado.
    /// Procesa la cámara y la envía entre hilos para que sistema cámaras pueda publicarla.
    fn create_camera_abm(&mut self) {
        if let Ok(camera) = self.create_camera(){
            self.process_and_send_camera(camera);
        }
    }

    /// Crea una cámara con el input proporcionado, y la devuelve.
    fn create_camera(&self) -> Result<Camera, Error> {

        let id = self.read_input_and_parse_to_u8("el ID")?;
        let latitude = self.read_input_and_parse_to_f64("la latitud")?;
        let longitude = self.read_input_and_parse_to_f64("la longitud")?;
        let range = self.read_input_and_parse_to_u8("el rango")?;

        Ok(Camera::new(id, latitude, longitude, range))
    }

    /// Lee el input de teclado y devuelve el valor parseado a u8.
    /// No debería fallar porque en caso de input inválido repregunta hasta obtener un input válido, pero devuelve un result.
    fn read_input_and_parse_to_u8(&self, pm_name: &str) -> Result<u8, Error> {
        let mut res: Result<u8, _> = self
            .get_input_abm(Some(format!("Ingrese {} de la cámara: ", pm_name).as_str()))
            .parse();

        while res.is_err() {
            res = self
            .get_input_abm(Some(format!("Error, intente nuevamente. Ingrese {} de la cámara: ", pm_name).as_str()))
            .parse();
        }

        if let Ok(value) = res {
            return Ok(value);
        }

        Err(Error::new(std::io::ErrorKind::InvalidInput, "Error al parsear u8 (no debería darse)."))
    }

    /// Lee el input de teclado y devuelve el valor parseado a f64.
    /// No debería fallar porque en caso de input inválido repregunta hasta obtener un input válido, pero devuelve un result.
    fn read_input_and_parse_to_f64(&self, pm_name: &str) -> Result<f64, Error> {
        let mut res: Result<f64, _> = self
            .get_input_abm(Some(format!("Ingrese {} de la cámara: ", pm_name).as_str()))
            .parse();

        while res.is_err() {
            res = self
            .get_input_abm(Some(format!("Error, intente nuevamente. Ingrese {} de la cámara: ", pm_name).as_str()))
            .parse();
        }

        if let Ok(value) = res {
            return Ok(value);
        }

        Err(Error::new(std::io::ErrorKind::InvalidInput, "Error al parsear f64 (no debería darse)."))
    }

    /// Obtiene el input por teclado.
    fn get_input_abm(&self, prompt: Option<&str>) -> String {
        if let Some(p) = prompt {
            print!("{}", p);
        }
        let _ = stdout().flush();
        let mut input = String::new();
        stdin()
            .read_line(&mut input)
            .expect("Error al leer la entrada");
        input.trim().to_string()
    }

    /// Procesa una nueva cámara (la inserta en el hashmap de cameras, maneja las lindantes), y la envía por un
    /// channel para que desde el rx el sistema cámaras le pueda hacer publish. Además, logguea la operación.
    fn process_and_send_camera(&mut self, new_camera: Camera) {
        match self.cameras.lock() {
            Ok(mut cams) => {
                // Recorre las cámaras ya existentes, agregando la nueva cámara como lindante de la que corresponda y viceversa, terminando la creación
                for camera in cams.values_mut() {
                    camera.mutually_add_if_bordering(&mut new_camera.clone());
                }
                self.logger.log(format!("Sistema-Camaras: envió cámara: {:?}", new_camera));
                // Envía la nueva cámara por tx, para ser publicada por el otro hilo
                if self.camera_tx.send(new_camera.to_bytes()).is_err() {
                    println!("Error al enviar cámara por tx desde hilo abm.");
                }
                // Guarda la nueva cámara
                cams.insert(new_camera.get_id(), new_camera);
                println!("Cámara agregada con éxito.\n");
            }
            Err(e) => println!("Error tomando lock en agregar cámara abm, {:?}.\n", e),
        }
    }

    /// Opción Mostrar cámaras del abm. Lista todas las cámaras existentes.
    fn show_cameras_abm(&self) {
        // Mostramos todas las cámaras
        println!("Cámaras registradas:\n");
        match self.cameras.lock() {
            Ok(cams) => {
                for camera in (*cams).values() {
                    // Si no está marcada borrada, mostrarla
                    if camera.is_not_deleted() {
                        camera.display();
                    };
                }
            }
            Err(_) => println!("Error al tomar lock de cámaras."),
        }
    }

    /// Opción Eliminar cámara, del abm.
    /// Elimina la cámara indicada, manejando sus lindantes, y la envía por tx para que rx haga publish.
    fn delete_camera_abm(&self) {
        if let Ok(id) = self.read_input_and_parse_to_u8("el ID") {
            self.delete_camera(id);
        }
    }

    /// Elimina a la cámara del id recibido.
    fn delete_camera(&self, id: u8) {
        match self.cameras.lock() {
            Ok(mut cams) => {
                if let Some(mut camera_to_delete) = cams.remove(&id) {
                    if camera_to_delete.is_not_deleted() {
                        camera_to_delete.delete_camera();

                        // Recorre las cámaras ya existentes, eliminando la cámara a eliminar como lindante de la que corresponda, terminando la eliminación
                        for camera in cams.values_mut() {
                            camera.remove_from_list_if_bordering(&mut camera_to_delete);
                        }

                        // Envía por el tx la cámara a eliminar para que se publique desde el otro hilo
                        // (con eso es suficiente. Si bien se les eliminó una lindante, no es necesario publicar el cambio
                        // de las demás ya que eso solo es relevante para sistema camaras)
                        if self.camera_tx.send(camera_to_delete.to_bytes()).is_err() {
                            println!("Error al enviar cámara por tx desde hilo abm.");
                        } else {
                            println!("Cámara eliminada con éxito.\n");
                        }
                    };
                } else {
                    println!("La cámara no existe.\n");
                }
            }
            Err(e) => println!("Error tomando lock baja abm, {:?}.\n", e),
        };
    }

    /// Opción Salir, del abm.
    fn exit_program_abm(&self) {
        match self.exit_tx.send(true) {
            Ok(_) => println!("Saliendo del programa."),
            Err(e) => println!("Error al intentar salir: {:?}", e),
        }
    }

    /// Recorre las cámaras y envía cada una por el channel, para que quien lea del rx haga el publish.
    fn send_cameras_from_file_to_publish(&self) {
        match self.cameras.lock() {
            Ok(cams) => {
                for camera in (*cams).values() {
                    println!("Iniciando, enviando cámara: {:?}", camera);
                    self.send_camera_bytes(camera, &self.camera_tx);
                }
            }
            Err(_) => println!("Error al tomar lock de cámaras."),
        }
    }

    /// Envía la cámara recibida, por el channel, para que quien la reciba por rx haga el publish.
    /// Además logguea la operación.
    fn send_camera_bytes(&self, camera: &Camera, camera_tx: &Sender<Vec<u8>>) {
        self.logger
            .log(format!("Sistema-Camaras: envío cámara: {:?}", camera));

        if camera_tx.send(camera.to_bytes()).is_err() {
            println!("Error al enviar cámara por tx desde hilo abm.");
            self.logger
                .log("Sistema-Camaras: error al enviar cámara por tx desde hilo abm.".to_string());
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::HashMap,
        sync::{mpsc, Arc, Mutex},
    };

    use crate::{apps::sist_camaras::camera::Camera, logging::string_logger::StringLogger};

    use super::ABMCameras;

    fn create_abm() -> ABMCameras {
        // Unos tx irrelevantes, para pasar al new de abm
        // (es necesario conservar las variables de rx en el test de todas formas, para que no se cierre el channel antes del assert)
        let (camera_tx, _camera_rx) = mpsc::channel();
        let (exit_tx, _exit_rx) = mpsc::channel();

        // Se crea el abm con su cameras
        let cameras = Arc::new(Mutex::new(HashMap::new()));
        // Se crea el logger
        //let (logger, logger_handle) = StringLogger::create_logger(String::from("Sistema-Cámaras")); // se usa con esto
        let (string_logger_tx, _string_logger_rx) = mpsc::channel(); // pero para testing, con esto.
        let logger_for_testing = StringLogger::new(string_logger_tx);
        
        ABMCameras::new(cameras.clone(), camera_tx, exit_tx, logger_for_testing)
    }

    #[test]
    fn test_1_abm_alta_de_camara_la_agrega_a_cameras() {
        
        let mut abm = create_abm();

        // Se agrega la cámara
        let new_camera_id = 1;
        let camera = Camera::new(new_camera_id, -34.0, -58.0, 5);
        abm.process_and_send_camera(camera);

        // Se busca la cámara recién agregada
        let mut is_new_cam_stored = false;
        if let Ok(cams) = abm.cameras.lock() {
            is_new_cam_stored = cams.contains_key(&new_camera_id);
        }
        // La cámara nueva se ha agregado a cameras
        assert!(is_new_cam_stored);
    }

    #[test]
    fn test_2_abm_baja_de_camara_la_elimina_de_cameras() {
        
        let mut abm = create_abm();

        // Se agrega la cámara
        let camera_to_remove_id = 1;
        let camera = Camera::new(camera_to_remove_id, -34.0, -58.0, 5);
        abm.process_and_send_camera(camera);

        // Ahora se la elimina
        abm.delete_camera(camera_to_remove_id);

        // Se busca la cámara recién eliminada
        let mut is_cam_to_remove_stored = false;
        if let Ok(cams) = abm.cameras.lock() {
            is_cam_to_remove_stored = cams.contains_key(&camera_to_remove_id);
        }
        // La cámara nueva se ha agregado a cameras
        assert!(!is_cam_to_remove_stored);
    }
}
