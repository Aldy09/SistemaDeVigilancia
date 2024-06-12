use std::{collections::HashMap, fs};

use super::camera::Camera;

pub fn read_cameras_from_file(filename: &str) -> HashMap<u8, Camera> {
    let mut cameras: HashMap<u8, Camera> = HashMap::new();
    let contents = fs::read_to_string(filename).expect("Error al leer el archivo de properties");

    for line in contents.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() == 5 {
            // Lee los atributos a cargar a la nueva cámara
            let id: u8 = parts[0].trim().parse().expect("Id no válido");
            let latitude = parts[1].trim().parse().expect("Latitud no válida");
            let longitude = parts[2].trim().parse().expect("Longitud no válida");
            let range = parts[3].trim().parse().expect("Rango no válido");
            let border_cam: u8 = parts[4].trim().parse().expect("Id no válido"); // ignorar la lindante por un momento, ahora borramos esto del arch.
            let border_cams_vec = vec![]; //vec![border_cam];

            let mut new_camera = Camera::new(id, latitude, longitude, range, border_cams_vec);

            // Recorre las cámaras ya existentes, agregando la nueva cámara como lindante de la que corresponda y viceversa, terminando la creación
            for camera in cameras.values_mut() {
                camera.mutually_add_if_bordering(&mut new_camera);                
            }

            // Guarda la nueva cámara
            cameras.insert(id, new_camera);
        }
    }

    cameras
}
