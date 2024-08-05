
// Funciones que realizan cálculos matemáticos.

pub fn calculate_distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}

/// Calcula la dirección en la que debe volar desde una posición `origin` hasta `destination`.
// Aux: esto estaría mejor en un struct posicion quizás? [] ver.
pub fn calculate_direction(origin: (f64, f64), destination: (f64, f64)) -> (f64, f64) {
    // calcular la distancia ('en diagonal') entre los dos puntos
    let (origin_lat, origin_lon) = (origin.0, origin.1);
    let (dest_lat, dest_lon) = (destination.0, destination.1);

    // Cálculo de distancia
    let lat_dist = dest_lat - origin_lat;
    let lon_dist = dest_lon - origin_lon;
    let distance = f64::sqrt(lat_dist.powi(2) + lon_dist.powi(2));

    // Vector unitario: (destino - origen) / || distancia ||, para cada coordenada.
    let unit_lat = lat_dist / distance;
    let unit_lon = lon_dist / distance;
    let direction: (f64, f64) = (unit_lat, unit_lon);

    direction
}