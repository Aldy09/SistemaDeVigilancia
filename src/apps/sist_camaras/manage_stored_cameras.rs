use std::{collections::HashMap, fs, sync::{Arc, Mutex}};

use super::camera::Camera;

/// Crea el hashmap de cámaras bien inicializado envuelto en un arc mutex, listo para ser usado
/// por sistema cámaras y sus módulos.
pub fn create_cameras() -> Arc<Mutex<HashMap<u8, Camera>>> {
    let cameras: HashMap<u8, Camera> = read_cameras_from_file("./cameras.properties");
    Arc::new(Mutex::new(cameras))
}

/// Lee las cámaras desde el archivo `filename`, las parsea y las crea, configurando también cuáles
/// son lindantes entre sí. Devuelve un hashmap con el id de cada cámara como clave y la cámara como valor.
fn read_cameras_from_file(filename: &str) -> HashMap<u8, Camera> {
    let mut cameras: HashMap<u8, Camera> = HashMap::new();
    let contents = fs::read_to_string(filename).expect("Error al leer el archivo de properties");

    for line in contents.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() == 4 {
            // Lee los atributos a cargar a la nueva cámara
            let id: u8 = parts[0].trim().parse().expect("Id no válido");
            let latitude = parts[1].trim().parse().expect("Latitud no válida");
            let longitude = parts[2].trim().parse().expect("Longitud no válida");
            let range = parts[3].trim().parse().expect("Rango no válido"); // []

            let mut new_camera = Camera::new(id, latitude, longitude, range);

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
