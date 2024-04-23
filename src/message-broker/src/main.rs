// Importa las bibliotecas necesarias
use log::{info, warn, error};
use env_logger;

fn main() {
    // Configura el sistema de registro
    env_logger::init();

    // Ejemplo de uso de registros
    info!("Este es un mensaje de información.");
    warn!("Esto es una advertencia.");
    error!("Esto es un error crítico.");
}
