extern crate reqwest;
extern crate image;

use std::fs::File;
use std::io::BufWriter;
use image::io::Reader as ImageReader;
use image::ImageFormat;
use std::io::Write;

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
    let img = ImageReader::open(temp_file).unwrap().decode().unwrap();
    img.show().unwrap();
}
