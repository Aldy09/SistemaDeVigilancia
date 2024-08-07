//! Few common places in the city of Wrocław, used in the example app.

use super::vendor::Position;

pub fn obelisco() -> Position {
    Position::from_lon_lat(-58.3861838, -34.6037344)
}

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
