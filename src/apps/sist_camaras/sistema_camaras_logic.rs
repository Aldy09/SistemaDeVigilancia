use std::{
    collections::HashMap,
    sync::{mpsc::Sender, MutexGuard},
};

use crate::{apps::incident_data::incident::Incident, logging::string_logger::StringLogger};

use crate::apps::sist_camaras::{
    camera::Camera,
    types::{hashmap_incs_type::HashmapIncsType, shareable_cameras_type::ShCamerasType},
};

#[derive(Debug)]
pub struct CamerasLogic {
    cameras: ShCamerasType,
    incs_being_managed: HashmapIncsType,
    cameras_tx: Sender<Vec<u8>>,
    logger: StringLogger,
}

impl CamerasLogic {
    /// Crea un struct CamerasLogic con las cámaras pasadas como parámetro e incidentes manejándose vacíos.
    pub fn new(cameras: ShCamerasType, cameras_tx: Sender<Vec<u8>>, logger: StringLogger) -> Self {
        Self {
            cameras,
            incs_being_managed: HashMap::new(),
            cameras_tx,
            logger,
        }
    }

    /// Procesa un Incidente recibido.
    pub fn manage_incident(
        &mut self,
        incident: Incident,
    ) {
        // Proceso los incidentes
        if !self.incs_being_managed.contains_key(&incident.get_info()) {
            self.process_first_time_incident(incident);
        } else {
            self.process_known_incident(incident);
        }
    }

    // Aux: (condición "hasta que" del enunciado).
    /// Procesa un incidente cuando un incidente con ese mismo id ya fue recibido anteriormente.
    /// Si su estado es resuelto, vuelve el estado de la/s cámara/s que lo atendían, a ahorro de energía.
    fn process_known_incident(
        &mut self,
        inc: Incident,
    ) {
        if inc.is_resolved() {
            self.logger.log(format!(
                "Recibo el inc {} de nuevo, ahora con estado resuelto.",
                inc.get_id()
            ));
            // Busco la/s cámara/s que atendían este incidente
            if let Some(cams_managing_inc) = self.incs_being_managed.get(&inc.get_info()) {
                // sé que existe, por el if de más arriba

                // Cambio el estado de las cámaras que lo manejaban, otra vez a ahorro de energía
                // solamente si el incidente en cuestión era el único que manejaban (si tenía más incidentes en rango, sigue estando activa)
                for camera_id in cams_managing_inc {
                    match self.cameras.lock() {
                        Ok(mut cams) => {
                            // Actualizo las cámaras en cuestión
                            if let Some(cam_to_update) = cams.get_mut(camera_id) {
                                let state_has_changed =
                                    cam_to_update.remove_from_incs_being_managed(inc.get_info());
                                self.logger.log(format!(
                                    "  la cámara queda: cam id y lista de incs: {:?}",
                                    cam_to_update.get_id_and_incs_for_debug_display()
                                ));
                                if state_has_changed {
                                    self.logger.log(format!(
                                        "Cambiado estado a Active, enviando cám: {:?}",
                                        cam_to_update
                                    ));
                                    self.send_camera_bytes(cam_to_update, &self.cameras_tx);
                                }
                            }
                        }
                        Err(_) => println!(
                            "Error al tomar lock de cámaras para volver estado a ahorro energía."
                        ),
                    };
                }
            }
            // También elimino la entrada del hashmap que busca por incidente, ya no le doy seguimiento
            self.incs_being_managed.remove(&inc.get_info());
        }
    }

    /// Procesa un incidente cuando el mismo fue recibido por primera vez.
    /// Para cada cámara ve si inc.pos está dentro de alcance de dicha cámara o sus lindantes,
    /// en caso afirmativo, se encarga de lo necesario para que la cámara y sus lindanes cambien su estado a activo.
    fn process_first_time_incident(
        &mut self,
        inc: Incident,
    ) {
        match self.cameras.lock() {
            Ok(mut cams) => {
                println!("Proceso el incidente {:?} por primera vez", inc.get_info());
                self.logger.log(format!(
                    "Proceso el incidente {:?} por primera vez",
                    inc.get_info()
                ));
                let cameras_that_follow_inc =
                    self.get_id_of_cams_that_will_change_state_to_active(&mut cams, &inc);

                // El vector tiene los ids de todas las cámaras que deben cambiar a activo
                for cam_id in &cameras_that_follow_inc {
                    if let Some(bordering_cam) = cams.get_mut(cam_id) {
                        // Agrega el inc a la lista de incs de la cámara, y de sus lindantes, para facilitar que luego puedan volver a su anterior estado
                        let state_has_changed =
                            bordering_cam.append_to_incs_being_managed(inc.get_info());
                        if state_has_changed {
                            self.logger.log(format!(
                                "Cambiando a estado Saving, enviando cám: {:?}",
                                bordering_cam
                            ));
                            self.send_camera_bytes(bordering_cam, &self.cameras_tx);
                        }
                    };
                }
                // Y se guarda las cámaras que le dan seguimiento al incidente, para luego poder encontrarlas fácilmente sin recorrer
                self.incs_being_managed.insert(inc.get_info(), cameras_that_follow_inc);
            }
            Err(_) => todo!(),
        }
    }

    /// Devuelve un vector de u8 con los ids de todas las cámaras que darán seguimiento al incidente `inc`.
    fn get_id_of_cams_that_will_change_state_to_active(
        &self,
        cams: &mut MutexGuard<'_, HashMap<u8, Camera>>,
        inc: &Incident,
    ) -> Vec<u8> {
        let mut cameras_that_follow_inc = vec![];

        // Recorremos cada una de las cámaras, para ver si el inc está en su rango
        for (cam_id, camera) in cams.iter_mut() {
            if camera.will_register(inc.get_position()) {
                self.logger.log(format!(
                    "Está en rango de cam: {}, cambiando su estado a activo.",
                    cam_id
                ));

                cameras_that_follow_inc.push(*cam_id);

                for bordering_cam_id in camera.get_bordering_cams() {
                    cameras_that_follow_inc.push(*bordering_cam_id);
                }
                self.logger.log(format!(
                    "  la cámara queda: cam id y lista de incs: {:?}",
                    camera.get_id_and_incs_for_debug_display()
                ));
            }
        }

        cameras_that_follow_inc
    }

    /// Envía la cámara recibida, por el channel, para que quien la reciba por rx haga el publish.
    /// Además logguea la operación.
    fn send_camera_bytes(&self, camera: &Camera, cameras_tx: &Sender<Vec<u8>>) {
        self.logger
            .log(format!("Sistema-Camaras: envío cámara: {:?}", camera));

        if cameras_tx.send(camera.to_bytes()).is_err() {
            println!("Error al enviar cámara por tx desde hilo abm.");
            self.logger
                .log("Sistema-Camaras: error al enviar cámara por tx desde hilo abm.".to_string());
        }
    }
}
