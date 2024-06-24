use crate::apps::sist_camaras::camera_state::CameraState;

#[derive(Debug, PartialEq)]
/// Struct que representa el estado de una de las cámaras del sistema central de cámaras.
/// Tiene:
/// - id;
/// - coordenadas x e y;
/// - estado;
/// - rango dentro del cual interesará manejar incidentes, es simllar a un radio pero que se suma a cada una de sus coordenadas;
/// - border_cameras: vector con los ids de sus cámaras lindantes;
/// - deleted: campo que indica si la Camera ha pasado por un borrado lógico en el sistema central de cámaras;
/// - incs_being_managed: vector con los ids de los incidentes a los que la Camera está prestando atención, esto es, ids de los incidentes que ocasionan que esta Camera esté en estado activo.
#[derive(Clone)]
pub struct Camera {
    id: u8,
    latitude: f64,
    longitude: f64,
    state: CameraState,
    range: u8,
    border_cameras: Vec<u8>,
    deleted: bool,
    incs_being_managed: Vec<u8>, // ids de los incidentes a los que está prestando atención
}

impl Camera {
    pub fn new(id: u8, latitude: f64, longitude: f64, range: u8) -> Self {
        Self {
            id,
            latitude,
            longitude,
            state: CameraState::SavingMode,
            range,
            border_cameras: vec![],
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
        let latitude = f64::from_be_bytes([
            bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
        ]);
        let longitude = f64::from_be_bytes([
            bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15], bytes[16],
        ]);
        let state = CameraState::from_byte([bytes[17]]);
        let range = bytes[18];
        let border_cameras_len = bytes[19];
        let mut border_cameras = vec![];
        for i in 0..border_cameras_len {
            border_cameras.push(bytes[20 + i as usize]);
        }
        let deleted = bytes[20 + border_cameras_len as usize] == 1;
        Self {
            id,
            latitude,
            longitude,
            state,
            range,
            border_cameras,
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
    pub fn will_register(&self, (latitude, longitude): (f64, f64)) -> bool {
        //hacer que la funcion retorne true si el incidente esta en el rango de la camara
        self.is_within_range_from_self(latitude, longitude, self.range as f64)
    }

    /// Modifica su estado al recibido por parámetro, y se marca un atributo
    /// para luego ser detectada como modificada y enviada.
    pub fn set_state_to(&mut self, new_state: CameraState) {
        self.state = new_state;
    }

    /// Devuelve un vector con los ids de sus cámaras lindantes.
    pub fn get_bordering_cams(&mut self) -> &mut Vec<u8> {
        //self.border_cameras.to_vec()
        &mut self.border_cameras
    }

    /// Agrega el inc_id a su lista de incidentes a los que le presta atención,
    /// y se cambia el estado a activo. Maneja su marcado.
    /// Devuelve si cambió su estado interno (a Activo).
    pub fn append_to_incs_being_managed(&mut self, inc_id: u8) -> bool {
        let mut state_has_changed = false;
        self.incs_being_managed.push(inc_id);
        // Si ya estaba en estado activo, la dejo como estaba (para no marcarla como modificada)
        if self.state != CameraState::Active {
            self.set_state_to(CameraState::Active);
            state_has_changed = true;
        };
        state_has_changed
    }

    /// Elimina el inc_id de su lista de incidentes a los que les presta atención,
    /// y si ya no le quedan incidentes, se cambia el estado a modo ahorro de energía.
    /// Maneja su marcado.
    /// Devuelve si cambió su estado interno (a Ahorro de energía).
    pub fn remove_from_incs_being_managed(&mut self, inc_id: u8) -> bool {
        let mut state_has_changed = false;
        if let Some(pos_de_inc_id) = self.incs_being_managed.iter().position(|&x| x == inc_id) {
            self.incs_being_managed.remove(pos_de_inc_id);
            // Maneja su lista y se cambiarse el estado si corresponde
            if self.incs_being_managed.is_empty() {
                self.set_state_to(CameraState::SavingMode);
                state_has_changed = true;
            }
        }
        state_has_changed
    }

    /// Función getter utilizada con propósitos de debugging.
    pub fn get_id_e_incs_for_debug_display(&self) -> (u8, Vec<u8>) {
        (self.id, self.incs_being_managed.to_vec())
    }

    /// Devuelve si la cámara ha pasado o no por un borrado lógico.
    pub fn is_not_deleted(&self) -> bool {
        !self.deleted
    }

    /// Hace un borrado lógico de la cámara, y como ello implica una modificación,
    /// se marca como no enviada.
    pub fn delete_camera(&mut self) {
        self.deleted = true;
    }

    /// Devuelve la latitud de la cámara.
    pub fn get_latitude(&self) -> f64 {
        self.latitude
    }

    /// Devuelve la longitud de la cámara.
    pub fn get_longitude(&self) -> f64 {
        self.longitude
    }

    pub fn get_id(&self) -> u8 {
        self.id
    }

    pub fn get_state(&self) -> CameraState {
        self.state
    }

    // Analiza si se encuentra la cámara recibida por parámetro dentro del border_range, en caso afirmativo:
    // tanto self como la cámara recibida por parámetro agregan sus ids mutuamente a la lista de lindantes de la otra.
    pub fn mutually_add_if_bordering(&mut self, candidate_bordering: &mut Camera) {
        let const_border_range: f64 = 4.0; // Constante que debe ir en arch de configuración.
        // Se fija si están en rango de lindantes.
        let in_range = self.is_within_range_from_self(
            candidate_bordering.get_latitude(),
            candidate_bordering.get_longitude(),
            const_border_range,
        );

        // Si sí, se agregan mutuamente como lindantes
        if in_range {
            self.border_cameras.push(candidate_bordering.get_id());
            candidate_bordering.border_cameras.push(self.id);
        }
    }

    pub fn remove_from_list_if_bordering(&mut self, camera_to_delete: &mut Camera) {
        // Aux: en realidad no necesito recalcular esto para borrarla; "si es lindante" en este contexto es "si está en la lista".
        //let const_border_range: f64 = 5.0; // Constante que debe ir en arch de configuración.
        //let in_range = self.is_within_range_from_self(camera_to_delete.get_latitude(), camera_to_delete.get_longitude(), const_border_range);

        //if in_range {
        // Busco la pos del id de la camera_to_delete en mi lista de lindantes, y la elimino
        if let Some(pos) = self
            .border_cameras
            .iter()
            .position(|id| *id == camera_to_delete.get_id())
        {
            self.border_cameras.remove(pos);
        }
        //}
    }

    /// Calcula si se encuentra las coordenadas pasadas se encuentran dentro del rango pasado
    fn is_within_range_from_self(&self, latitude: f64, longitude: f64, range: f64) -> bool {
        let lat_dist = self.latitude - latitude;
        let long_dist = self.longitude - longitude;
        let rad = f64::sqrt(lat_dist.powi(2) + long_dist.powi(2));

        //let adjusted_range = range / 400.0; // hay que modificar el range de las cámaras, ahora que son latitudes de verdad y no "3 4".
                                                 // println!("Dio que la cuenta vale: {}, y adj_range vale: {}", rad, adjusted_range); // debug []

        let adjusted_range = 0.00135 + 0.0012 * range;
        // aux dron: let adjusted_range = range / 1000.0; // hay que modificar el range de las cámaras, ahora que son latitudes de verdad y no "3 4".
        println!(
            "Dio que la cuenta vale: {}, y adj_range vale: {}. Era rango: {}",
            rad, adjusted_range, range
        ); // debug []
        //println!()
        rad <= (adjusted_range)
    }
}

#[cfg(test)]

mod test {
    use super::Camera;

    #[test]
    fn test_1_camera_to_y_from_bytes() {
        let camera = Camera::new(12, 3.0, 4.0, 5);

        let bytes = camera.to_bytes();

        let camera_reconstruida = Camera::from_bytes(&bytes);

        assert_eq!(camera_reconstruida, camera);
    }

    #[test]
    fn test_2_camaras_cercanas_son_lindantes() {
        //     Aux: obelisco: lon -58.3861838  lat: -34.6037344

        let lat = -34.6037344;
        let lon = -58.3861838;
        let range = 10;
        let incr = 0.0000005;
        let mut cam_1 = Camera::new(1, lat, lon, range);

        // Otra cámara, con misma longitud, y latitud apenas incrementada
        let mut cam_2 = Camera::new(2, lat + incr, lon, range);

        cam_1.mutually_add_if_bordering(&mut cam_2);
        // Aux con estos datos da: Dio que la cuenta vale: 0.0000004999999987376214

        // Se han agregado mutuamente, xq sí qentraron dentro del border_range para ser consideradas lindantes
        assert!(cam_1.border_cameras.contains(&cam_2.get_id()));
        assert!(cam_2.border_cameras.contains(&cam_1.get_id()));

        //
        // Ídem con datos "reales"
        let mut cam_5: Camera = Camera::new(5, -34.6040, -58.3873, 1); // Aux: cámara 5.
        let mut cam_6: Camera = Camera::new(6, -34.6039, -58.3837, 1); // Aux: cámara 6.
        
        cam_5.mutually_add_if_bordering(&mut cam_6);

        // Se han agregado mutuamente, xq sí qentraron dentro del border_range para ser consideradas lindantes
        assert!(cam_5.border_cameras.contains(&cam_6.get_id()));
        assert!(cam_6.border_cameras.contains(&cam_5.get_id()));

    }

    #[test]
    fn test_3_camaras_lejanas_no_son_lindantes() {
        /*//     Aux: obelisco: lon -58.3861838  lat: -34.6037344
        let lat = -34.6037344;
        let lon = -58.3861838;
        let range = 10;
        let incr = 0.0000005;*/
        //let mut cam_1 = Camera::new(1, lat, lon, range);
        // A más de cuatro cuadras: -58.3938 -34.6044
        //let mut cam_a: Camera = Camera::new(10, -34.6044, -58.3938, 1); // 4 cuadras a la izq de cam 5.
        // 58.3954 -34.6045
        //let mut cam_a: Camera = Camera::new(10, -34.4045, -58.1954, 1); // RE lejos

        // -58.3907 -34.6043
        //let mut cam_a: Camera = Camera::new(10, -34.6043, -58.3907, 1); // 2 cuadras a la izq de cam 5.
        
        // -58.3920 -34.6044
        //let mut cam_a: Camera = Camera::new(10, -34.6044, -58.3920, 1); // 3 cuadras a la izq de cam 5.

        //5 cuadras: o sea afuera de las 4 cuadras de lindantes
        //-58.3950 -34.6044
        let mut cam_a: Camera = Camera::new(10, -34.6044, -58.3950, 1); // 3 cuadras a la izq de cam 5.

        // Otra cámara, con misma longitud, y latitud MUY incrementada
        //let mut cam_2 = Camera::new(2, lat + 10.0 * incr, lon, range);
        //let mut cam_b: Camera = Camera::new(4, -34.6042, -58.3909, 1); // Aux: cámara 4.
        let mut cam_b: Camera = Camera::new(5, -34.6040, -58.3873, 1); // Aux: cámara 5.

        //cam_1.mutually_add_if_bordering(&mut cam_2);
        cam_b.mutually_add_if_bordering(&mut cam_a);
        // Aux con estos datos da: Dio que la cuenta vale: 0.0000004999999987376214

        // No se han agregado mutuamente, xq no entraron dentro del border_range para ser consideradas lindantes
        assert!(!cam_a.border_cameras.contains(&cam_b.get_id()));
        assert!(!cam_b.border_cameras.contains(&cam_a.get_id()));
    }

    // #[test]
    // fn test_4_testing_camera_range() {

    //     let camera = Camera::new(5, -34.6040, -58.3873, 3); // Aux: cámara 5.
        
    //     let (lat, lon) = (-34.6042, -58.3897);
    //     let is_in_range = camera.is_within_range_from_self(lat, lon, camera.range as f64);
        
    //     assert!(is_in_range);
        
    // }

    #[test]
    fn test_4a_una_pos_dentro_de_range_cuadras_esta_dentro_del_rango() {

        // Rango de 1 cuadra.
        let camera = Camera::new(5, -34.6040, -58.3873, 1); // Aux: cámara 5.
        
        let (lat, lon) = (-34.6042, -58.3897); // una cuadra a la izq de la cam 5
        let is_in_range = camera.is_within_range_from_self(lat, lon, camera.range as f64);
        
        assert!(is_in_range);
        //assert!(false);
        
    }

    #[test]
    fn test_4b_una_pos_mas_lejana_esta_fuera_del_rango() {

        // Rango de 1 cuadra.
        let camera = Camera::new(5, -34.6040, -58.3873, 1); // Aux: cámara 5.
        
        let (lat, lon) = (-34.6042, -58.3902);
        let is_in_range = camera.is_within_range_from_self(lat, lon, camera.range as f64);
        
        assert!(!is_in_range);
        
    }
}
