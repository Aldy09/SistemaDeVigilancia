use log::{info, warn, error};

fn main() {
    env_logger::init();

    info!("Este es un mensaje de información.");
    warn!("Esto es una advertencia.");
    error!("Esto es un error crítico.");
}
