extern crate reqwest;
extern crate image;

use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

use image::GenericImageView;
use minifb::{Window, WindowOptions};

fn main() {
    // URL del mapa estático de Yandex Maps
    let map_url = "http://static-maps.yandex.ru/1.x/?lang=en-US&ll=32.810152,39.889847&size=450,450&z=10&l=map&pt=32.810152,39.889847,pm2rdl1~32.870152,39.869847,pm2rdl99";

    // Realizar la solicitud GET para obtener la imagen del mapa
    let response = reqwest::blocking::get(map_url).unwrap();
    let image_bytes = response.bytes().unwrap();

    // Guardar la imagen en un archivo temporal (opcional)
    let temp_file = "mapa_temp.png";
    let output_file = File::create(temp_file).unwrap();
    let mut writer = BufWriter::new(output_file);
    writer.write_all(&image_bytes).unwrap();

    // Mostrar la imagen por pantalla utilizando la biblioteca image
    //let img = ImageReader::open(temp_file).unwrap().decode().unwrap();
    //img.show().unwrap();

    let img = image::open(temp_file).unwrap();
    let img_dimensions = img.dimensions();
    let img_buffer_u8 = img.to_rgba8().into_raw();

    // Convertir el buffer de u8 a u32
    let img_buffer_u32: Vec<u32> = img_buffer_u8.chunks(4).map(|b| u32::from_ne_bytes([b[0], b[1], b[2], b[3]])).collect();

    let mut window = Window::new(
        "Visualizador de imágenes",
        img_dimensions.0 as usize,
        img_dimensions.1 as usize,
        WindowOptions::default(),
    ).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        window.update_with_buffer(&img_buffer_u32, img_dimensions.0 as usize, img_dimensions.1 as usize).unwrap();
}
    
}
