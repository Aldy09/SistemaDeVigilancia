extern crate staticmap;
extern crate image;
extern crate glium;

use staticmap::{Map, Marker, Point, Size};
use std::fs::File;
use std::io::BufWriter;
use image::ImageBuffer;
use glium::{DisplayBuild, Surface};

fn main() {
    // Crear el mapa estático de OpenStreetMap
    let map = Map::new()
        .set_size(Size::new(800, 600)) // Tamaño de la imagen del mapa
        .set_center(Point::new(-25.363, 131.044)) // Coordenadas para el centro del mapa
        .set_zoom(10) // Nivel de zoom del mapa
        .add_marker(Marker::new(Point::new(-25.363, 131.044))) // Marca en el centro del mapa (opcional)
        .set_language("en"); // Idioma del mapa (opcional)

    // Guardar el mapa en un archivo de imagen (opcional)
    let output_file = File::create("mapa.png").unwrap();
    let mut writer = BufWriter::new(output_file);
    map.save(&mut writer).unwrap();

    // Mostrar la imagen del mapa en una ventana con glium
    let display = glium::glutin::WindowBuilder::new().build_glium().unwrap();
    let map_image = image::open("mapa.png").unwrap().to_rgba8();
    let dimensions = map_image.dimensions();
    let image_rgba = glium::texture::RawImage2d::from_raw_rgba_reversed(&map_image.into_raw(), dimensions);
    let map_texture = glium::texture::Texture2d::new(&display, image_rgba).unwrap();

    // Loop principal de glium para mostrar la imagen
    'main: loop {
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0); // Limpiar la ventana con color negro

        // Dibujar la imagen del mapa en la ventana
        let map_size = map_texture.get_dimensions();
        glium::draw(&target, &glium::vertex::VertexBuffer::empty_dynamic(&display, 4).unwrap(), &glium::index::NoIndices(glium::index::PrimitiveType::TriangleStrip), &glium::program::Program::from_source(&display, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC, None).unwrap(), &glium::uniforms::UniformsStorage::new("tex", glium::uniforms::Sampler::new(&map_texture).magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)), &Default::default()).unwrap();

        target.finish().unwrap();

        // Manejar eventos y salir del loop si la ventana es cerrada
        for event in display.poll_events() {
            match event {
                glium::glutin::Event::Closed => break 'main,
                _ => (),
            }
        }
    }
}

// Código del shader
const VERTEX_SHADER_SRC: &str = r#"
    #version 140

    in vec2 position;
    in vec2 tex_coords;
    out vec2 v_tex_coords;

    void main() {
        gl_Position = vec4(position, 0.0, 1.0);
        v_tex_coords = tex_coords;
    }
"#;

const FRAGMENT_SHADER_SRC: &str = r#"
    #version 140

    uniform sampler2D tex;
    in vec2 v_tex_coords;
    out vec4 color;

    void main() {
        color = texture(tex, v_tex_coords);
    }
"#;
