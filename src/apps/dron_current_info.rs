/// Struct que representa a cada uno de los drones del sistema de vigilancia.

/// Struct que contiene los campos que identifican al dron (el id) y que pueden modificarse durante su funcionamiento-
#[derive(Debug, PartialEq)]
pub struct DronCurrentInfo {
    // #Pensando, quizás convenga que estos 5 atributos estén en un struct aparte, xq solo estos son los
    // que vamos a mandar en un publish, ya que las de más abajo son todas constantes que no interesa enviar.
    id: u8,
    // Posición actual
    latitude: f64,
    longitude: f64,
    battery_lvl: u8,
    state: u8, // esto en realidad es un enum, volver []
}

#[allow(dead_code)]
impl DronCurrentInfo {
    /// Dron se inicia con batería al 100%
    /// Aux: desde la posición de mantenimiento, y vuela hacia el range_center (por ejemplo).
    /// Aux: Otra posibilidad sería que inicie desde la pos del range_center. <-- hacemos esto, por simplicidad con los estados por ahora.
    /// Se inicia con estado []. Ver (el activo si ya está en el range_center, o ver si inicia en mantenimiento).
    pub fn new(id: u8) -> Self {
        // Acá puede cargar las constantes desde archivo de config.
        let range_center_lat_property = -34.20;
        let range_center_lon_property = -58.20;

        DronCurrentInfo {
            id,
            latitude: range_center_lat_property, // Inicia desde el range_center.
            longitude: range_center_lon_property,
            battery_lvl: 100,
            state: 1,
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self.id.to_be_bytes());
        bytes.extend_from_slice(&self.latitude.to_be_bytes());
        bytes.extend_from_slice(&self.longitude.to_be_bytes());
        bytes.extend_from_slice(&self.battery_lvl.to_be_bytes());
        //bytes.push(self.state.to_byte()[0]);
        bytes.extend_from_slice(&self.state.to_be_bytes());
        bytes
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
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

        let state = u8::from_be_bytes([bytes[idx]]);
        //idx += b_size; // comentado porque warning is never read.

        DronCurrentInfo { id, latitude, longitude, battery_lvl, state }
    }
}

#[cfg(test)]
mod test {
    use crate::apps::dron_current_info::DronCurrentInfo;

    #[test]
    fn test_1_dron_to_y_from_bytes() {
        let dron = DronCurrentInfo { id: 1, latitude: -34.0, longitude: -58.0, battery_lvl: 100, state: 1 };
        
        let bytes = dron.to_bytes();
        let reconstructed_dron = DronCurrentInfo::from_bytes(bytes);
        
        assert_eq!(reconstructed_dron, dron);
    }
}