use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};


fn read_cameras_from_file(filename: &str) -> HashMap<String, (i32, i32)> {
    let mut cameras = HashMap::new();
    let contents = fs::read_to_string(filename).expect("Error al leer el archivo de properties");

    for line in contents.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() == 3 {
            let guid = parts[0].trim().to_string();
            let coord_x = parts[1].trim().parse().expect("Coordenada X no válida");
            let coord_y = parts[2].trim().parse().expect("Coordenada Y no válida");
            cameras.insert(guid, (coord_x, coord_y));
        }
    }

    cameras
}

fn main() {
    println!("SISTEMA DE CAMARAS\n");

    let mut cameras: HashMap<String, (i32, i32)> = read_cameras_from_file("cameras.properties");

    loop {
        println!("1. Agregar cámara");
        println!("2. Mostrar cámaras");
        println!("3. Salir");
        print!("Ingrese una opción: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Error al leer la entrada");

        match input.trim() {
            "1" => {
                print!("Ingrese el GUID de la cámara: ");
                io::stdout().flush().unwrap();
                let mut guid = String::new();
                io::stdin().read_line(&mut guid).expect("Error al leer la entrada");

                print!("Ingrese la coordenada X: ");
                io::stdout().flush().unwrap();
                let mut coord_x = String::new();
                io::stdin().read_line(&mut coord_x).expect("Error al leer la entrada");
                let coord_x: i32 = coord_x.trim().parse().expect("Coordenada X no válida");

                print!("Ingrese la coordenada Y: ");
                io::stdout().flush().unwrap();
                let mut coord_y = String::new();
                io::stdin().read_line(&mut coord_y).expect("Error al leer la entrada");
                let coord_y: i32 = coord_y.trim().parse().expect("Coordenada Y no válida");

                cameras.insert(guid.trim().to_string(), (coord_x, coord_y));
                println!("Cámara agregada con éxito.\n");
            }
            "2" => {
                println!("Cámaras registradas:\n");
                for (guid, (coord_x, coord_y)) in &cameras {
                    println!("GUID: {}", guid);
                    println!("Coordenada X: {}", coord_x);
                    println!("Coordenada Y: {}\n", coord_y);
                }
            }
            "3" => {
                println!("Saliendo del programa.");
                break;
            }
            _ => {
                println!("Opción no válida. Intente nuevamente.\n");
            }
        }
    }
}
