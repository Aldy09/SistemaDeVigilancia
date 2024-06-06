use super::dron_current_info::DronCurrentInfo;

/// Struct que representa a cada uno de los drones del sistema de vigilancia.
/// Al publicar en el topic `dron`, solamente el struct `DronCurrentInfo` es lo que interesa enviar,
/// ya que lo demás son constantes para el funcionamiento del Dron.
#[derive(Debug, PartialEq)]
pub struct Dron {
    // El id y su posición y estado actuales se encuentran en el siguiente struct
    current_info: DronCurrentInfo,

    // Y a continuación, constantes cargadas desde un arch de configuración
    max_battery_lvl: u8,
    min_operational_battery_lvl: u8,
    range: u8,
    stay_at_inc_time: u8, // Tiempo a permanencer en la ubicación del incidente, desde la llegada, en segundos.
    // Range center, porque un dron se mueve, al terminar de atender incidente vuelve a este range center
    range_center_lat: f64, // Aux: #ToDo: Capaz es mejor tener una Posicion, para no tener mil f64s sueltos []
    range_center_lon: f64,
    // Posicion de la central, para volver a cargarse la batería cuando se alcanza el min_operational_battery_lvl
    mantainance_lat: f64,
    mantainance_lon: f64,
}

#[allow(dead_code)]
impl Dron {
    /// Dron se inicia con batería al 100%
    /// Inicia desde la pos del range_center, con estado activo. <-- Aux: hacemos esto, por simplicidad con los estados por ahora.
    /// (Aux: otra posibilidad era que inicie desde la posición de mantenimiento, y vuele hacia el range_center; pero ahí ya ver en qué estado iniciaría)
    pub fn new(id: u8) -> Self {
        // Acá puede cargar las constantes desde archivo de config.
        let range_center_lat_property = -34.20;
        let range_center_lon_property = -58.20;

        // Inicia desde el range_center, por lo cual tiene estado 1 (activo); y con batería al 100%.
        let current_info = DronCurrentInfo::new(
            id,
            range_center_lat_property,
            range_center_lon_property,
            100,
            1,
        );

        Dron {
            current_info,
            // Las siguientes son las constantes, que vienen del arch de config:
            max_battery_lvl: 100,
            min_operational_battery_lvl: 20,
            range: 40,
            stay_at_inc_time: 200,
            range_center_lat: range_center_lat_property,
            range_center_lon: range_center_lon_property,
            mantainance_lat: -34.30,
            mantainance_lon: -58.30,
        }
    }
}
