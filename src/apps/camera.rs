use crate::apps::camera_state::CameraState;

#[derive(Debug)]
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
    coord_x: u8,
    coord_y: u8,
    state: CameraState,
    range: u8,
    border_cameras: Vec<u8>,
    pub sent: bool,
    pub deleted: bool,
    incs_being_managed: Vec<u8>, // ids de los incidentes a los que está prestando atención
}

impl Camera {
    pub fn new(id: u8, coord_x: u8, coord_y: u8, range: u8, border_cameras: Vec<u8>) -> Self {
        Self {
            id,
            coord_x,
            coord_y,
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
        bytes.extend_from_slice(&self.coord_x.to_be_bytes());
        bytes.extend_from_slice(&self.coord_y.to_be_bytes());
        bytes.extend_from_slice(&self.state.to_byte());
        bytes.push(self.range);
        bytes.extend_from_slice(&(self.border_cameras.len() as u8).to_be_bytes());
        for camera in &self.border_cameras {
            bytes.push(*camera);
        }
        bytes.push(self.deleted as u8);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let id = bytes[0];
        let coord_x = bytes[1];
        let coord_y = bytes[2];
        let state = CameraState::from_byte([bytes[3]]);
        let range = bytes[4];
        let border_cameras_len = bytes[5];
        let mut border_cameras = vec![];
        for i in 0..border_cameras_len {
            border_cameras.push(bytes[6 + i as usize]);
        }
        let deleted = bytes[12 + border_cameras_len as usize] == 1;
        Self {
            id,
            coord_x,
            coord_y,
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
        println!("Coordenada X: {}", self.coord_x);
        println!("Coordenada Y: {}", self.coord_y);
        println!("Estado: {:?}", self.state);
        println!("Rango de alcance: {}", self.range);
        println!("Cámaras lindantes: {:?}\n", self.border_cameras);
    }

    /// Devuelve si el incidente de coordenadas `(inc_coord_x, inc_coord_y)`
    /// está en el rango de la cámara `Self`.
    pub fn will_register(&self, (inc_coord_x, inc_coord_y): (u8, u8)) -> bool {
        let is_in_x_range = self.coord_x + self.range >= inc_coord_x; // El range es un radio
        let is_in_y_range = self.coord_y + self.range >= inc_coord_y;

        is_in_x_range & is_in_y_range
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
}
