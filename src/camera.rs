use crate::camera_state::CameraState;

pub struct Camera {
    id: u8,
    coord_x: i32,
    coord_y: i32,
    state: CameraState,
    range: u8,
    border_cameras: Vec<u8>,
}

impl Camera {
    pub fn new(id: u8, coord_x: i32, coord_y: i32, range: u8, border_cameras: Vec<u8>) -> Self {
        Self {
            id,
            coord_x,
            coord_y,
            state: CameraState::SavingMode,
            range,
            border_cameras,
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
}
