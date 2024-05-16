use std::fs::File;
use std::io::{self, Write};
use std::fmt::Write as FmtWrite;
use uuid::Uuid;
use rand::Rng;



fn main() -> io::Result<()> {
    let mut file = File::create("cameras.properties")?;

    writeln!(file, "# Lista de cámaras")?;
    writeln!(file, "# Formato: GUID:X:Y")?;
    writeln!(file)?;

    for _ in 0..10 {
        let uuid = Uuid::new_v4();
        let mut rng = rand::thread_rng();
        let x: u32 = rng.gen_range(1..20);
        let y: u32 = rng.gen_range(1..20);
        writeln!(file, "{}:{}:{}", uuid, x, y)?;
    }

    println!("Archivo de propiedades generado correctamente.");

    Ok(())
}
