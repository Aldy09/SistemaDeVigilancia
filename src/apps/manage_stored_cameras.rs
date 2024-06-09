use std::{collections::HashMap, fs};

use super::camera::Camera;

pub fn read_cameras_from_file(filename: &str) -> HashMap<u8, Camera> {
    let mut cameras = HashMap::new();
    let contents = fs::read_to_string(filename).expect("Error al leer el archivo de properties");

    for line in contents.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() == 5 {
            let id: u8 = parts[0].trim().parse().expect("Id no válido");
            let latitude = parts[1].trim().parse().expect("Latitud no válida");
            let longitude = parts[2].trim().parse().expect("Longitud no válida");
            let range = parts[3].trim().parse().expect("Rango no válido");
            let border_cam: u8 = parts[4].trim().parse().expect("Id no válido");
            let vec = vec![border_cam];

            let camera = Camera::new(id, latitude, longitude, range, vec);
            cameras.insert(id, camera);
        }
    }

    cameras
}
