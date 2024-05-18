#[derive(Debug)]
pub enum CameraState {
    Active,
    SavingMode,
}


impl CameraState {
    pub fn to_byte(&self) -> [u8; 1] {
        match self {
            CameraState::Active => (1 as u8).to_be_bytes(),
            CameraState::SavingMode => (2 as u8).to_be_bytes(),
        }
    }

    pub fn from_byte(bytes: [u8; 1]) -> Self {
        match u8::from_be_bytes(bytes) {
            1 => CameraState::Active,
            2 => CameraState::SavingMode,
            _ => panic!("Estado de cámara no válido"),
        }
    }
}

