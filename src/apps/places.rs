// Lugares de interés en distintas ciudades.

use super::vendor::Position;

// Obelisco, Buenos Aires, Argentina.
pub fn obelisco() -> Position {
    Position::from_lon_lat(-58.3861838, -34.6037344)
}

/// Lugar de carga, cuando los drones se queda sin batería.
pub fn mantenimiento() -> Position {
    Position::from_lon_lat(-58.3816, -34.6037)
}


pub fn dworcowa_bus_stop() -> Position {
    Position::from_lon_lat(17.03940, 51.10005)
}


pub fn capitol() -> Position {
    Position::from_lon_lat(17.03018, 51.10073)
}

pub fn wroclavia() -> Position {
    Position::from_lon_lat(17.03471, 51.09648)
}
