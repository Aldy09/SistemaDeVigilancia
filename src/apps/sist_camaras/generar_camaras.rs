use rand::Rng;
use std::fs::File;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let mut file = File::create("./cameras.properties")?;

    writeln!(file, "# Lista de cámaras")?;
    writeln!(
        file,
        "# Formato: ID:X:Y:RANGE:BORDER_CAM_ID
    "
    )?;
    writeln!(file)?;

    for i in 0..10 {
        let id: u8 = i;
        let mut rng = rand::thread_rng();
        let latitude: f64 = rng.gen_range(-34.6047344..-34.6027350);
        let longitude: f64 = rng.gen_range(-58.3871838..-58.3851850);

        let range: u8 = 3;
        let border_cameras: u8 = rng.gen_range(0..9); /* Ver */
        /*border_cameras,*/
        writeln!(
            file,
            "{}:{}:{}:{}:{}",
            id, latitude, longitude, range, border_cameras
        )?;
    }

    println!("Archivo de propiedades generado correctamente.");

    Ok(())
}
