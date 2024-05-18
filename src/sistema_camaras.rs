use rustx::camera::Camera;
use std::collections::HashMap;
use std::io::{self, Write};
use std::{fs, thread};

fn read_cameras_from_file(filename: &str) -> HashMap<u8, Camera> {
    let mut cameras = HashMap::new();
    let contents = fs::read_to_string(filename).expect("Error al leer el archivo de properties");

    for line in contents.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() == 5 {
            let id: u8 = parts[0].trim().parse().expect("Id no válido");
            let coord_x = parts[1].trim().parse().expect("Coordenada X no válida");
            let coord_y = parts[2].trim().parse().expect("Coordenada Y no válida");
            let range = parts[3].trim().parse().expect("Rango no válido");
            let border_cam: u8 = parts[4].trim().parse().expect("Id no válido");
            let vec = vec![border_cam];

            //println!("[DEBUG]")
            let camera = Camera::new(id, coord_x, coord_y, range, vec);
            cameras.insert(id, camera);
        }
    }

    cameras
}

fn main() {
    println!("SISTEMA DE CAMARAS\n");

    let mut cameras: HashMap<u8, Camera> = read_cameras_from_file("cameras.properties");

    // Menú cámaras
    let handle = thread::spawn(move || {
        abm_cameras(&mut cameras);
    });

    // Manejar incidentes

    // Esperar hijo
    if handle.join().is_err() {
        println!("Error al esperar al hijo.");
    }
}

fn abm_cameras(cameras: &mut HashMap<u8, Camera>) {
    loop {
        println!("1. Agregar cámara");
        println!("2. Mostrar cámaras");
        println!("3. Eliminar cámara");
        println!("4. Salir");
        print!("Ingrese una opción: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Error al leer la entrada");

        match input.trim() {
            "1" => {
                print!("Ingrese el ID de la cámara: ");
                io::stdout().flush().unwrap();
                let mut read_id = String::new();
                io::stdin()
                    .read_line(&mut read_id)
                    .expect("Error al leer la entrada");
                let id: u8 = read_id.trim().parse().expect("Coordenada X no válida");

                print!("Ingrese la coordenada X: ");
                io::stdout().flush().unwrap();
                let mut read_coord_x = String::new();
                io::stdin()
                    .read_line(&mut read_coord_x)
                    .expect("Error al leer la entrada");
                let coord_x: i32 = read_coord_x.trim().parse().expect("Coordenada X no válida");

                print!("Ingrese la coordenada Y: ");
                io::stdout().flush().unwrap();
                let mut read_coord_y = String::new();
                io::stdin()
                    .read_line(&mut read_coord_y)
                    .expect("Error al leer la entrada");
                let coord_y: i32 = read_coord_y.trim().parse().expect("Coordenada Y no válida");

                print!("Ingrese el rango: ");
                io::stdout().flush().unwrap();
                let mut read_range = String::new();
                io::stdin()
                    .read_line(&mut read_range)
                    .expect("Error al leer la entrada");
                let range: u8 = read_range.trim().parse().expect("Rango no válido");

                print!("Ingrese el id de cámara lindante: ");
                io::stdout().flush().unwrap();
                let mut read_border_cam = String::new();
                io::stdin()
                    .read_line(&mut read_border_cam)
                    .expect("Error al leer la entrada");
                let border_camera: u8 = read_border_cam.trim().parse().expect("Id no válido");

                let new_camera = Camera::new(id, coord_x, coord_y, range, vec![border_camera]);
                cameras.insert(id, new_camera);
                println!("Cámara agregada con éxito.\n");
            }
            "2" => {
                println!("Cámaras registradas:\n");
                for camera in (*cameras).values() {
                    camera.display();
                }
            }
            "3" => {
                print!("Ingrese el ID de la cámara a eliminar: ");
                io::stdout().flush().unwrap();
                let mut read_id = String::new();
                io::stdin()
                    .read_line(&mut read_id)
                    .expect("Error al leer la entrada");
                let id: u8 = read_id.trim().parse().expect("Id no válido");

                match cameras.remove(&id) {
                    Some(_) => println!("Cámara eliminada con éxito.\n"),
                    None => println!("No se encontró la cámara con el ID especificado.\n"),
                }
            }
            "4" => {
                println!("Saliendo del programa.");
                break;
            }
            _ => {
                println!("Opción no válida. Intente nuevamente.\n");
            }
        }
    }
}
