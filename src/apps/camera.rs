use crate::apps::camera_state::CameraState;

#[derive(Debug, PartialEq)]
/// Struct que representa el estado de una de las cámaras del sistema central de cámaras.
/// Tiene:
/// - id;
/// - coordenadas x e y;
/// - estado;
/// - rango dentro del cual interesará manejar incidentes, es simllar a un radio pero que se suma a cada una de sus coordenadas;
/// - border_cameras: vector con los ids de sus cámaras lindantes;
/// - sent: campo que indica si la Camera se envió y aún no se modificó (`sent=true`) o si por el contrario modificó desde la última vez que el sistema central de cámaras la envió (`sent=false`);
/// - deleted: campo que indica si la Camera ha pasado por un borrado lógico en el sistema central de cámaras;
/// - incs_being_managed: vector con los ids de los incidentes a los que la Camera está prestando atención, esto es, ids de los incidentes que ocasionan que esta Camera esté en estado activo.
pub struct Camera {
    id: u8,
    latitude: f32,
    longitude: f32,
    state: CameraState,
    range: u8,
    border_cameras: Vec<u8>,
    sent: bool, // false si se modificó, true al enviarla
    deleted: bool,
    incs_being_managed: Vec<u8>, // ids de los incidentes a los que está prestando atención
}

impl Camera {
    pub fn new(id: u8, latitude: f32, longitude: f32, range: u8, border_cameras: Vec<u8>) -> Self {
        Self {
            id,
            latitude,
            longitude,
            state: CameraState::SavingMode,
            range,
            border_cameras,
            sent: false,
            deleted: false,
            incs_being_managed: vec![],
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.push(self.id);
        bytes.extend_from_slice(&self.latitude.to_be_bytes());
        bytes.extend_from_slice(&self.longitude.to_be_bytes());
        bytes.extend_from_slice(&self.state.to_byte());
        bytes.extend_from_slice(&self.range.to_be_bytes());
        bytes.extend_from_slice(&(self.border_cameras.len() as u8).to_be_bytes());
        for camera in &self.border_cameras {
            bytes.push(*camera);
        }
        bytes.push(self.deleted as u8);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let id = bytes[0];
        let latitude = f32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        let longitude = f32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
        let state = CameraState::from_byte([bytes[9]]);
        let range = bytes[10];
        let border_cameras_len = bytes[11];
        let mut border_cameras = vec![];
        for i in 0..border_cameras_len {
            border_cameras.push(bytes[12 + i as usize]);
        }
        let deleted = bytes[12 + border_cameras_len as usize] == 1;
        Self {
            id,
            latitude,
            longitude,
            state,
            range,
            border_cameras,
            sent: false,
            deleted,
            incs_being_managed: vec![],
        }
    }

    pub fn display(&self) {
        println!("ID: {}", self.id);
        println!("Latitude: {}", self.latitude);
        println!("Longitude: {}", self.longitude);
        println!("Estado: {:?}", self.state);
        println!("Rango de alcance: {}", self.range);
        println!("Cámaras lindantes: {:?}\n", self.border_cameras);
    }

    /// Devuelve si el incidente de coordenadas `(inc_coord_x, inc_coord_y)`
    /// está en el rango de la cámara `Self`.
    pub fn will_register(&self, (latitude, longitude): (f32, f32)) -> bool {
        //hacer que la funcion retorne true si el incidente esta en el rango de la camara
        let x = self.latitude - latitude;
        let y = self.longitude - longitude;
        let distance = (x.powi(2) + y.powi(2)).sqrt();
        distance <= self.range as f32
    }

    /// Modifica su estado al recibido por parámetro, y se marca un atributo
    /// para luego ser detectada como modificada y enviada.
    pub fn set_state_to(&mut self, new_state: CameraState) {
        self.state = new_state;
        self.sent = false;
    }

    /// Devuelve un vector con los ids de sus cámaras lindantes.
    pub fn get_bordering_cams(&self) -> Vec<u8> {
        self.border_cameras.to_vec()
    }

    /// Agrega el inc_id a su lista de incidentes a los que le presta atención,
    /// y se cambia el estado a activo. Maneja su marcado.
    pub fn append_to_incs_being_managed(&mut self, inc_id: u8) {
        self.incs_being_managed.push(inc_id);
        // Si ya estaba en estado activo, la dejo como estaba (para no marcarla como modificada)
        if self.state != CameraState::Active {
            self.set_state_to(CameraState::Active);
        };
    }

    /// Elimina el inc_id de su lista de incidentes a los que les presta atención,
    /// y si ya no le quedan incidentes, se cambia el estado a modo ahorro de energía.
    /// Maneja su marcado.
    pub fn remove_from_incs_being_managed(&mut self, inc_id: u8) {
        if let Some(pos_de_inc_id) = self.incs_being_managed.iter().position(|&x| x == inc_id) {
            self.incs_being_managed.remove(pos_de_inc_id);
            // Maneja su lista y se cambiarse el estado si corresponde
            if self.incs_being_managed.is_empty() {
                self.set_state_to(CameraState::SavingMode);
            }
        }
    }

    /// Función getter utilizada con propósitos de debugging.
    pub fn get_id_e_incs_for_debug_display(&self) -> (u8, Vec<u8>) {
        (self.id, self.incs_being_managed.to_vec())
    }

    /// Devuelve true si la cámara ha sido modificada desde la última vez que fue enviada,
    /// y por lo tanto debería ser enviada,
    /// o false si no se ha modificado desde el último envío.
    /// Si nunca se produce un envío, esta función devolverá true.
    pub fn modified_after_last_sent(&self) -> bool {
        !self.sent
    }
    /// Marca a la cámara como enviada.
    pub fn marked_as_sent(&mut self) {
        self.sent = true;
    }
    /// Devuelve si la cámara ha pasado o no por un borrado lógico.
    pub fn is_not_deleted(&self) -> bool {
        !self.deleted
    }

    /// Hace un borrado lógico de la cámara, y como ello implica una modificación,
    /// se marca como no enviada.
    pub fn delete_camera(&mut self) {
        self.deleted = true;
        self.sent = false;
    }

    /// Devuelve la latitud de la cámara.
    pub fn get_latitude(&self) -> f32 {
        self.latitude
    }

    /// Devuelve la longitud de la cámara.
    pub fn get_longitude(&self) -> f32 {
        self.longitude
    }

    pub fn get_id(&self) -> u8 {
        self.id
    }
}

#[cfg(test)]

mod test {
    use super::Camera;

    #[test]
    fn test_1_camera_to_y_from_bytes() {
        let camera = Camera::new(12, 3.0, 4.0, 5, vec![6]);

        let bytes = camera.to_bytes();

        let camera_reconstruida = Camera::from_bytes(&bytes);

        assert_eq!(camera_reconstruida, camera);
    }
    // #[test]
    // fn text_will_register() {
    //     let camera = Camera::new(12, 3.0, 4.0, 5, vec![6]);
    //     assert_eq!(camera.will_register((3.0, 4.0)), true);
    //     assert_eq!(camera.will_register((3.0, 4.1)), true);
    //     assert_eq!(camera.will_register((3.0, 4.2)), true);
    //     assert_eq!(camera.will_register((3.0, 4.3)), true);
    //     assert_eq!(camera.will_register((3.0, 4.4)), true);
    //     assert_eq!(camera.will_register((3.0, 4.5)), false);
    //     assert_eq!(camera.will_register((3.0, 4.6)), false);
    //     assert_eq!(camera.will_register((3.0, 4.7)), false);
    //     assert_eq!(camera.will_register((3.0, 4.8)), false);
    //     assert_eq!(camera.will_register((3.0, 4.9)), false);
    // }
}
