/// Struct que representa a cada uno de los drones del sistema de vigilancia.
pub struct Dron {
    id: u8,
    // Posición actual
    latitude: f64,
    longitude: f64,
    battery_lvl: u8,
    state: u8, // esto en realidad es un enum, volver []

    // A continuación, son constantes cargadas desde un arch de configuración
    max_battery_lvl: u8,
    min_operational_battery_lvl: u8,
    range: u8,
    stay_at_inc_time: u8, // Tiempo a permanencer en la ubicación del incidente, desde la llegada.
    // Range center, porque un dron se mueve, al terminar de atender incidente vuelve a este range center
    range_center_lat: f64, // Aux: #ToDo: Capaz es mejor tener una Posicion, para no tener mil f64s sueltos []
    range_center_lon: f64,
    // Posicion de la central, para volver a cargarse la batería
    mantainance_lat: f64,
    mantainance_lon: f64,
}