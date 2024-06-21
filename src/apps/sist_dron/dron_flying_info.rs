use std::io::Error;

/// Dirección y velocidad con las que vuela el dron.
#[derive(Debug, PartialEq)]
pub struct DronFlyingInfo {
    direction: (f64, f64), // vector unitario de dirección al volar, con componentes lat y lon
    speed: f64,            // velocidad de desplazamiento al volar
}

impl DronFlyingInfo {
    pub fn new(direction: (f64, f64), speed: f64) -> Self {
        DronFlyingInfo { direction, speed }
    }

    /// Pasa un struct `DronFlyingInfo` a bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        // direction
        bytes.extend_from_slice(&self.direction.0.to_be_bytes());
        bytes.extend_from_slice(&self.direction.1.to_be_bytes());

        // speed
        bytes.extend_from_slice(&self.speed.to_be_bytes());

        bytes
    }

    /// Obtiene un struct `DronFlyingInfo` a partir de bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        let mut idx = 0;
        let b_size: usize = 1;

        let latitude = f64::from_be_bytes([
            bytes[idx],
            bytes[idx + b_size],
            bytes[idx + 2 * b_size],
            bytes[idx + 3 * b_size],
            bytes[idx + 4 * b_size],
            bytes[idx + 5 * b_size],
            bytes[idx + 6 * b_size],
            bytes[idx + 7 * b_size],
        ]);
        idx += 8 * b_size;

        let longitude = f64::from_be_bytes([
            bytes[idx],
            bytes[idx + b_size],
            bytes[idx + 2 * b_size],
            bytes[idx + 3 * b_size],
            bytes[idx + 4 * b_size],
            bytes[idx + 5 * b_size],
            bytes[idx + 6 * b_size],
            bytes[idx + 7 * b_size],
        ]);
        idx += 8 * b_size;
        let direction = (latitude, longitude);

        // Leo la velocidad
        let speed = f64::from_be_bytes([
            bytes[idx],
            bytes[idx + b_size],
            bytes[idx + 2 * b_size],
            bytes[idx + 3 * b_size],
            bytes[idx + 4 * b_size],
            bytes[idx + 5 * b_size],
            bytes[idx + 6 * b_size],
            bytes[idx + 7 * b_size],
        ]);
        // idx += 8 * b_size; //idx += b_size; // comentado porque warning is never read. quizás en el futuro agregamos más campos.

        Ok(DronFlyingInfo { direction, speed })
    }

    /// Devuelve dirección (lat, lon) y velocidad de vuelo actuales.
    pub fn get_flying_info_values(&self) -> ((f64, f64), f64) {
        (self.direction, self.speed)
    }
}
