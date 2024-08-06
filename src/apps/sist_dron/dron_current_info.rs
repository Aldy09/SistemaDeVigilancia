use std::io::{Error, ErrorKind};

use crate::apps::incident_data::incident_info::IncidentInfo;

use super::dron_flying_info::DronFlyingInfo;
use super::dron_state::DronState;

/// Struct que contiene los campos que identifican al Dron (el id) y que pueden modificarse durante su funcionamiento.
#[derive(Debug, PartialEq, Clone)]
pub struct DronCurrentInfo {
    id: u8,
    // Posición actual
    latitude: f64,
    longitude: f64,
    battery_lvl: u8,
    state: DronState,
    inc_info_to_resolve: Option<IncidentInfo>,
    // Dirección y velocidad de vuelo
    flying_info: Option<DronFlyingInfo>,
}

#[allow(dead_code)]
impl DronCurrentInfo {
    /// Inicia con los parámetros recibidos; con ningún incidente en resolución y sin flying_info
    /// (es decir, inicia con estos dos últimos atributos en None).
    pub fn new(id: u8, latitude: f64, longitude: f64, battery_lvl: u8, state: DronState) -> Self {
        DronCurrentInfo {
            id,
            latitude,
            longitude,
            battery_lvl,
            state,
            inc_info_to_resolve: None,
            flying_info: None,
        }
    }

    /// Pasa un struct `DronCurrentInfo` a bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self.id.to_be_bytes());
        bytes.extend_from_slice(&self.latitude.to_be_bytes());
        bytes.extend_from_slice(&self.longitude.to_be_bytes());
        //println!("BYTES ID LAT Y LONG, ENCODEANDO: {:?}", bytes); //aux [] debug
        bytes.extend_from_slice(&self.battery_lvl.to_be_bytes());
        //bytes.push(self.state.to_byte()[0]); // <-- así sería si fuera un enum en vez de un u8.
        bytes.extend_from_slice(&self.state.to_byte());

        // El info del incidente que se está resolviendo:
        let mut inc_info_to_send: Vec<u8> = vec![0,0];
        if let Some(inc_info) = &self.inc_info_to_resolve {
            inc_info_to_send = inc_info.to_bytes();
        }
        bytes.extend_from_slice(&inc_info_to_send);
        

        // La flying_info: dir y velocidad de vuelo
        if let Some(f) = &self.flying_info {
            bytes.extend_from_slice(&1_u8.to_be_bytes()); // avisa que se enviará algo más
            bytes.extend_from_slice(&f.to_bytes());
        } else {
            bytes.extend_from_slice(&0_u8.to_be_bytes()); // avisa que No se enviará más bytes
        }
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

        let mut inc_info_to_resolve = None;
        let inc_info_to_resolve_option = IncidentInfo::from_bytes([bytes[idx], bytes[idx+b_size]].to_vec())?;
        if let Some(inc_info) = inc_info_to_resolve_option {
            inc_info_to_resolve = Some(inc_info);
        }
        // Leo el inc id to resolve
        /*let mut inc_id_to_resolve = None;
        let read_inc_id = u8::from_be_bytes([bytes[idx]]);
        if read_inc_id != 0 {
            inc_id_to_resolve = Some(read_inc_id);
        }*/
        idx += 2 * b_size;

        // Leo dir y velocidad de vuelo
        let mut flying_info = None;
        let is_there_flying_info = u8::from_be_bytes([bytes[idx]]);
        idx += b_size;

        if is_there_flying_info == 1 {
            flying_info = Some(DronFlyingInfo::from_bytes(bytes[idx..].to_vec())?);
        }

        //idx += b_size; // comentado porque warning is never read. quizás en el futuro agregamos más campos.

        match state_res {
            Ok(state) => Ok(DronCurrentInfo {
                id,
                latitude,
                longitude,
                battery_lvl,
                state,
                inc_info_to_resolve,
                flying_info,
            }),
            Err(_) => Err(Error::new(
                ErrorKind::InvalidInput,
                "Error al leer el state",
            )),
        }
    }

    // Getters
    /// Devuelve el id.
    pub fn get_id(&self) -> u8 {
        self.id
    }
    /// Devuelve latitud y longitud en las que dron se encuentra actualmente.
    pub fn get_current_position(&self) -> (f64, f64) {
        (self.latitude, self.longitude)
    }
    /// Devuelve el nivel de batería actual.
    pub fn get_battery_lvl(&self) -> u8 {
        self.battery_lvl
    }
    /// Devuelve el estado en que dron se encuentra actualmente.
    pub fn get_state(&self) -> DronState {
        self.state
    }

    /// Setea el estado del dron.
    pub fn set_state(&mut self, new_state: DronState) {
        self.state = new_state;
    }

    /// Devuelve el id del incidente que el dron se encuentra actualmente resolviendo.
    pub fn get_inc_id_to_resolve(&self) -> Option<IncidentInfo> {
        self.inc_info_to_resolve
    }

    /// Setea el id del incidente que el dron se encuentra actualmente resolviendo.
    pub fn set_inc_id_to_resolve(&mut self, inc_info: IncidentInfo) {
        self.inc_info_to_resolve = Some(inc_info);
    }
    /// Borra el id del incidente resuelto
    pub fn unset_inc_id_to_resolve(&mut self) {
        self.inc_info_to_resolve = None;
    }

    /// Setea la flying_info recibida.
    pub fn set_flying_info(&mut self, info: DronFlyingInfo) {
        self.flying_info = Some(info);
    }

    /// Setea la posición actual del dron.
    pub fn set_current_position(&mut self, new_position: (f64, f64)) {
        self.latitude = new_position.0;
        self.longitude = new_position.1;
    }

    /// Incrementa la posición actual en la dirección recibida, y devuelve la nueva posición actual.
    pub fn increment_current_position_in(&mut self, dir: (f64, f64)) -> (f64, f64) {
        // La dirección es un vector unitario, pero para poder sumarlo a la lat y long y que tenga sentido
        // hay que escalarla.
        self.latitude += dir.0 / 10000.0;
        self.longitude += dir.1 / 10000.0;

        self.get_current_position()
    }

    /// Devuelve dirección (lat, lon) y velocidad de vuelo actuales, o None si el dron
    /// actualmente no está volando.
    pub fn get_flying_info(&self) -> Option<((f64, f64), f64)> {
        self.flying_info
            .as_ref()
            .map(|f| f.get_flying_info_values()) // <-- clippy
                                                 /*if let Some(f) = &self.flying_info {
                                                     Some(f.get_flying_info_values())
                                                 } else {
                                                     None
                                                 }*/
    }

    /// Establece `None` como `flying_info`, lo cual indica que el dron no está actualmente en desplazamiento.
    pub fn unset_flying_info(&mut self) {
        self.flying_info = None;
    }

    pub fn get_distance_to(&self, destination: (f64, f64)) -> f64 {
        let origin = self.get_current_position();
        let lat_dist = destination.0 - origin.0;
        let lon_dist = destination.1 - origin.1;
        f64::sqrt(lat_dist.powi(2) + lon_dist.powi(2))
    }

    /// Decrementa la batería, y chequea y devuelve si la batería está por debajo del mínimo.
    /// Si es menor que la minima establece el id del incidente a resolver en None.
    pub fn decrement_and_check_battery_lvl(&mut self, min_battery: u8) -> bool {
        let mut should_charge = false;
        // Decrementa
        if self.battery_lvl >= 5 {
            self.battery_lvl -= 5;
        } else {
            self.battery_lvl = 0;
        }
        // Analiza
        if self.battery_lvl < min_battery {
            should_charge = true;
            self.inc_info_to_resolve = None; // []
        }
        should_charge
    }

    pub fn set_battery_lvl(&mut self, new_lvl: u8) {
        self.battery_lvl = new_lvl;
    }
}

#[cfg(test)]
mod test {
    use crate::apps::sist_dron::{dron_current_info::DronCurrentInfo, dron_state::DronState};
    use crate::apps::incident_data::{incident_info::IncidentInfo, incident_source::IncidentSource};

    #[test]
    fn test_1a_dron_to_y_from_bytes() {
        let dron = DronCurrentInfo {
            id: 1,
            latitude: -34.0,
            longitude: -58.0,
            battery_lvl: 100,
            state: DronState::ExpectingToRecvIncident,
            inc_info_to_resolve: None,
            flying_info: None,
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
            inc_info_to_resolve: Some(IncidentInfo::new(18, IncidentSource::Manual)),
            flying_info: None,
        };

        let bytes = dron.to_bytes();
        let reconstructed_dron = DronCurrentInfo::from_bytes(bytes);

        assert_eq!(reconstructed_dron.unwrap(), dron);
    }
}
