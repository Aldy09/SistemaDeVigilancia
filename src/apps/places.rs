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

/// Taking a public bus (line 106) is probably the cheapest option to get from
/// the train station to the airport.
/// https://www.wroclaw.pl/en/how-and-where-to-buy-public-transport-tickets-in-wroclaw
pub fn dworcowa_bus_stop() -> Position {
    Position::from_lon_lat(17.03940, 51.10005)
}

/// Musical Theatre Capitol.
pub fn capitol() -> Position {
    Position::from_lon_lat(17.03018, 51.10073)
}

/// Shopping center, and the main intercity bus station.
pub fn wroclavia() -> Position {
    Position::from_lon_lat(17.03471, 51.09648)
}
