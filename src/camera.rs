struct Camera {
    id: u8,
    coord_x: i32,
    coord_y: i32,
    state: camera_state,
    range: u8,
    border_cameras: Vec<u8>,
}

impl Camera {
    fn new(id: u8, coord_x: i32, coord_y: i32, range: u8, border_cameras: Vec<u8>) -> Self {
        Self {
            id,
            coord_x,
            coord_y,
            state: CameraState::saving_mode,
            range,
            border_cameras,
        }
    }

    fn display(&self) {
        println!("ID: {}", self.guid);
        println!("Coordenada X: {}", self.coord_x);
        println!("Coordenada Y: {}\n", self.coord_y);
        println!("Estado: {}\n", self.state);
        println!("Rango de alcance: {}\n", self.range);
        println!("Cámaras lindantes: {:?}\n", self.border_cameras);
    }
}
