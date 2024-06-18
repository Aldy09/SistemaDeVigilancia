use std::io::{Error, ErrorKind};

use super::dron_state::DronState;

/// Struct que contiene los campos que identifican al Dron (el id) y que pueden modificarse durante su funcionamiento.
#[derive(Debug, PartialEq)]
pub struct DronCurrentInfo {
    id: u8,
    // Posición actual
    latitude: f64,
    longitude: f64,
    battery_lvl: u8,
    state: DronState, // esto en realidad es un enum, volver [].
}

#[allow(dead_code)]
impl DronCurrentInfo {
    /// Dron se inicia con batería al 100%
    /// Aux: desde la posición de mantenimiento, y vuela hacia el range_center (por ejemplo).
    /// Aux: Otra posibilidad sería que inicie desde la pos del range_center. <-- hacemos esto, por simplicidad con los estados por ahora.
    /// Se inicia con estado []. Ver (el activo si ya está en el range_center, o ver si inicia en mantenimiento).
    pub fn new(id: u8, latitude: f64, longitude: f64, battery_lvl: u8, state: DronState) -> Self {
        DronCurrentInfo {
            id,
            latitude,
            longitude,
            battery_lvl,
            state,
        }
    }

    /// Pasa un struct `DronCurrentInfo` a bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self.id.to_be_bytes());
        bytes.extend_from_slice(&self.latitude.to_be_bytes());
        bytes.extend_from_slice(&self.longitude.to_be_bytes());
        bytes.extend_from_slice(&self.battery_lvl.to_be_bytes());
        //bytes.push(self.state.to_byte()[0]); // <-- así sería si fuera un enum en vez de un u8.
        bytes.extend_from_slice(&self.state.to_byte());
        bytes
    }

    /// Obtiene un struct `DronCurrentInfo` a partir de bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        let mut idx = 0;
        let b_size: usize = 1;

        let id = u8::from_be_bytes([bytes[idx]]);
        idx += b_size;

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

        let battery_lvl = u8::from_be_bytes([bytes[idx]]);
        idx += b_size;

        let state_res = DronState::from_byte([bytes[idx]]);
        //idx += b_size; // comentado porque warning is never read. quizás en el futuro agregamos más campos.
        match state_res {
            Ok(state) => Ok(DronCurrentInfo {
                id,
                latitude,
                longitude,
                battery_lvl,
                state,
            }),
            Err(_) => Err(Error::new(ErrorKind::InvalidInput, "Error al leer el state")),
        }

        
    }

    // Getters
    /// Devuelve el id
    pub fn get_id(&self) -> u8 {
        self.id
    }
    /// Devuelve latitud y longitud en las que dron se encuentra actualmente
    pub fn get_current_position(&self) -> (f64, f64) {
        (self.latitude, self.longitude)
    }
    /// Devuelve el nivel de batería actual
    pub fn get_battery_lvl(&self) -> u8 {
        self.id
    }
    /// Devuelve el estado en que dron se encuentra actualmente
    pub fn get_state(&self) -> &DronState {
        &self.state
    }
    
    pub fn set_state(&mut self, new_state: DronState) {
        self.state = new_state;
    }
}

#[cfg(test)]
mod test {
    use crate::apps::{dron_current_info::DronCurrentInfo, dron_state::DronState};

    #[test]
    fn test_1_dron_to_y_from_bytes() {
        let dron = DronCurrentInfo {
            id: 1,
            latitude: -34.0,
            longitude: -58.0,
            battery_lvl: 100,
            state: DronState::ExpectingToRecvIncident,
        };

        let bytes = dron.to_bytes();
        let reconstructed_dron = DronCurrentInfo::from_bytes(bytes);

        assert_eq!(reconstructed_dron.unwrap(), dron);
    }
}
