use std::io::Error;

use super::{dron_current_info::DronCurrentInfo, sist_dron_properties::SistDronProperties};

/// Struct que representa a cada uno de los drones del sistema de vigilancia.
/// Al publicar en el topic `dron`, solamente el struct `DronCurrentInfo` es lo que interesa enviar,
/// ya que lo demás son constantes para el funcionamiento del Dron.
#[derive(Debug, PartialEq)]
pub struct Dron {
    // El id y su posición y estado actuales se encuentran en el siguiente struct
    current_info: DronCurrentInfo,

    // Y a continuación, constantes cargadas desde un arch de configuración
    dron_properties: SistDronProperties,
}

#[allow(dead_code)]
impl Dron {
    /// Dron se inicia con batería al 100%
    /// Inicia desde la pos del range_center, con estado activo. <-- Aux: hacemos esto, por simplicidad con los estados por ahora.
    /// (Aux: otra posibilidad era que inicie desde la posición de mantenimiento, y vuele hacia el range_center; pero ahí ya ver en qué estado iniciaría)
    pub fn new(id: u8) -> Result<Self, Error> {
        // Se cargan las constantes desde archivo de config.
        let properties_file = "src/apps/sistema_dron.properties";//"sistema_dron.properties";//"./sistema_dron.properties";
        let dron_properties = SistDronProperties::new(properties_file)?;

        // Inicia desde el range_center, por lo cual tiene estado 1 (activo); y con batería al 100%.
        let current_info = DronCurrentInfo::new(
            id,
            dron_properties.range_center_lat,
            dron_properties.range_center_lon,
            100,
            1,
        );

        Ok(Dron {
            current_info,
            // Las siguientes son las constantes, que vienen del arch de config:
            dron_properties,
            /*max_battery_lvl: 100,
            min_operational_battery_lvl: 20,
            range: 40,
            stay_at_inc_time: 200,
            range_center_lat: range_center_lat_property,
            range_center_lon: range_center_lon_property,
            mantainance_lat: -34.30,
            mantainance_lon: -58.30,*/
        })
    }
}

#[cfg(test)]

mod test {
    use super::Dron;

    #[test]
    fn test_1_dron_se_inicia_con_id_y_estado_correctos() {
        let dron = Dron::new(1).unwrap();
        
        assert_eq!(dron.current_info.get_id(), 1);
        assert_eq!(dron.current_info.get_state(), 1); // estado activo
    }


}