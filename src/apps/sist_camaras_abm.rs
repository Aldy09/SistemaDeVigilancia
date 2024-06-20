use std::{
    collections::HashMap,
    io::{stdin, stdout, Write},
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
};

use crate::{
    apps::app_type::AppType,
    structs_to_save_in_logger::{OperationType, StructsToSaveInLogger},
};

use super::camera::Camera;

pub struct ABMCameras {
    cameras: Arc<Mutex<HashMap<u8, Camera>>>,
    logger_tx: mpsc::Sender<StructsToSaveInLogger>,
    camera_tx: Sender<Vec<u8>>,
    exit_tx: Sender<bool>,
}

impl ABMCameras {
    /// Crea un struct `ABMCameras`.
    pub fn new(
        cameras: Arc<Mutex<HashMap<u8, Camera>>>,
        logger_tx: mpsc::Sender<StructsToSaveInLogger>,
        camera_tx: Sender<Vec<u8>>,
        exit_tx: Sender<bool>,
    ) -> Self {
        ABMCameras {
            cameras,
            logger_tx,
            camera_tx,
            exit_tx,
        }
    }

    /// Pone en funcionamiento el menú del abm para cámaras.
    /// Como cameras es un arc, quien haya llamado a esta función podrá ver reflejados los cambios.
    pub fn run(&mut self) {
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
        let camera = self.create_camera();
        self.process_and_send_camera(camera);
    }

    /// Crea una cámara con el input proporcionado, y la devuelve.
    fn create_camera(&self) -> Camera {
        let id: u8 = self
            .get_input_abm(Some("Ingrese el ID de la cámara: "))
            .parse()
            .expect("ID no válido");
        let latitude: f64 = self
            .get_input_abm(Some("Ingrese Latitud: "))
            .parse()
            .expect("Latitud no válida");
        let longitude: f64 = self
            .get_input_abm(Some("Ingrese Longitud: "))
            .parse()
            .expect("Longitud no válida");
        let range: u8 = self
            .get_input_abm(Some("Ingrese el rango: "))
            .parse()
            .expect("Rango no válido");

        Camera::new(id, latitude, longitude, range)
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
    fn process_and_send_camera(&mut self, camera_clone: Camera) {
        match self.cameras.lock() {
            Ok(mut cams) => {
                // Recorre las cámaras ya existentes, agregando la nueva cámara como lindante de la que corresponda y viceversa, terminando la creación
                for camera in cams.values_mut() {
                    camera.mutually_add_if_bordering(&mut camera_clone.clone());
                }
                //Envia la camara al log.txt
                self.logger_tx
                    .send(StructsToSaveInLogger::AppType(
                        "Sistema Camaras".to_string(),
                        AppType::Camera(camera_clone.clone()),
                        OperationType::Sent,
                    ))
                    .unwrap();
                // Envía la nueva cámara por tx, para ser publicada por el otro hilo
                if self.camera_tx.send(camera_clone.to_bytes()).is_err() {
                    println!("Error al enviar cámara por tx desde hilo abm.");
                }
                // Guarda la nueva cámara
                cams.insert(camera_clone.get_id(), camera_clone);
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
        let id: u8 = self
            .get_input_abm(Some("Ingrese el ID de la cámara a eliminar: "))
            .parse()
            .expect("Id no válido");
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
}

#[cfg(test)]
mod test {

    #[test]
    fn test_1_abm_alta_de_camara() {}
}
