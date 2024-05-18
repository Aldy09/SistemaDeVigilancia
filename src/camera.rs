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

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self.id.to_be_bytes());
        bytes.extend_from_slice(&self.coord_x.to_be_bytes());
        bytes.extend_from_slice(&self.coord_y.to_be_bytes());
        bytes.extend_from_slice(&self.state.to_byte());
        bytes.extend_from_slice(&self.range.to_be_bytes());
        bytes.extend_from_slice(&(self.border_cameras.len() as u8).to_be_bytes());
        for camera in &self.border_cameras {
            bytes.extend_from_slice(&camera.to_be_bytes());
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let id = u8::from_be_bytes([bytes[0]]);
        let coord_x = i32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        let coord_y = i32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
        let state = CameraState::from_byte([bytes[9]]);
        let range = u8::from_be_bytes([bytes[10]]);
        let border_cameras_len = u8::from_be_bytes([bytes[11]]);
        let mut border_cameras = vec![];
        for i in 0..border_cameras_len {
            border_cameras.push(u8::from_be_bytes([bytes[12 + i as usize]]));
        }
        Self {
            id,
            coord_x,
            coord_y,
            state,
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
