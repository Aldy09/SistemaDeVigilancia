use std::io::{Error, ErrorKind};

use super::dron_state::DronState;
use super::dron_flying_info::DronFlyingInfo;

/// Struct que contiene los campos que identifican al Dron (el id) y que pueden modificarse durante su funcionamiento.
#[derive(Debug, PartialEq)]
pub struct DronCurrentInfo {
    id: u8,
    // Posición actual
    latitude: f64,
    longitude: f64,
    battery_lvl: u8,
    state: DronState, // esto en realidad es un enum, volver [].
    inc_id_to_resolve: Option<u8>,
    // // Dirección y velocidad, flying_info
    // direction: (f64, f64), // vector unitario de dirección al volar, con componentes lat y lon
    // speed: Option<f64>, // velocidad de desplazamiento al volar
    flying_info: DronFlyingInfo,
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
            inc_id_to_resolve: None,
            flying_info: DronFlyingInfo::new(),
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

        // El id del incidente que se está resolviendo:
        let mut inc_id_to_send = 0;
        if let Some(inc_id) = self.inc_id_to_resolve {
            inc_id_to_send = inc_id;
        }
        bytes.extend_from_slice(&inc_id_to_send.to_be_bytes());
        bytes.extend_from_slice(&self.flying_info.to_bytes()); // dir y velocidad de vuelo
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
        idx += b_size;

        // Leo el inc id to resolve
        let mut inc_id_to_resolve = None;
        let read_inc_id = u8::from_be_bytes([bytes[idx]]);
        if read_inc_id != 0 {
            inc_id_to_resolve = Some(read_inc_id);
        }
        idx += b_size; // comentado porque warning is never read. quizás en el futuro agregamos más campos.
        
        // dir y velocidad de vuelo
        let flying_info = DronFlyingInfo::from_bytes(bytes[idx..].to_vec())?;

        match state_res {
            Ok(state) => Ok(DronCurrentInfo {
                id,
                latitude,
                longitude,
                battery_lvl,
                state,
                inc_id_to_resolve,
                flying_info,
            }),
            Err(_) => Err(Error::new(
                ErrorKind::InvalidInput,
                "Error al leer el state",
            )),
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

    /// Setea el estado del dron
    pub fn set_state(&mut self, new_state: DronState) {
        self.state = new_state;
    }

    /// Devuelve el id del incidente que el dron se encuentra actualmente resolviendo
    pub fn get_inc_id_to_resolve(&self) -> Option<u8> {
        self.inc_id_to_resolve
    }

    /// Setea el id del incidente que el dron se encuentra actualmente resolviendo
    pub fn set_inc_id_to_resolve(&mut self, inc_id: u8) {
        self.inc_id_to_resolve = Some(inc_id);
    }

    /// Incrementa la posición actual en la dirección recibida, y devuelve la nueva posición actual
    pub fn increment_current_position_in(&mut self, dir: (f64, f64)) -> (f64, f64) {
        self.latitude += dir.0;
        self.longitude += dir.1;

        self.get_current_position()
    }
}

#[cfg(test)]
mod test {
    use crate::apps::{dron_current_info::DronCurrentInfo, dron_flying_info::DronFlyingInfo, dron_state::DronState};

    #[test]
    fn test_1a_dron_to_y_from_bytes() {
        let dron = DronCurrentInfo {
            id: 1,
            latitude: -34.0,
            longitude: -58.0,
            battery_lvl: 100,
            state: DronState::ExpectingToRecvIncident,
            inc_id_to_resolve: None,
            flying_info: DronFlyingInfo::new(),
        };

        let bytes = dron.to_bytes();
        let reconstructed_dron = DronCurrentInfo::from_bytes(bytes);

        assert_eq!(reconstructed_dron.unwrap(), dron);
    }

    #[test]
    fn test_1b_dron_to_y_from_bytes() {
        let dron = DronCurrentInfo {
            id: 1,
            latitude: -34.0,
            longitude: -58.0,
            battery_lvl: 100,
            state: DronState::ExpectingToRecvIncident,
            inc_id_to_resolve: Some(18),
            flying_info: DronFlyingInfo::new(),
        };

        let bytes = dron.to_bytes();
        let reconstructed_dron = DronCurrentInfo::from_bytes(bytes);

        assert_eq!(reconstructed_dron.unwrap(), dron);
    }
}
