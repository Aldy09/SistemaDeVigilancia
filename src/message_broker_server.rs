use std::collections::HashMap;
use std::io::Error;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
type ShareableStream = Arc<Mutex<TcpStream>>;
type ShHashmapType = Arc<Mutex<HashMap<String, Vec<ShareableStream>>>>;
//use rustx::mqtt_server::{create_server, handle_incoming_connections, load_port};

fn main() -> Result<(), Error> {
    // env_logger::init();

    // let (ip, port) = load_port()?;

    // let listener = create_server(ip, port)?;

    // // Creo estructura subs_by_topic a usar (es un "Hashmap<topic, vec de subscribers>")
    // // No es único hilo! al subscribe y al publish en cuestión lo hacen dos clientes diferentes! :)
    // let subs_by_topic: ShHashmapType = Arc::new(Mutex::new(HashMap::new()));

    // handle_incoming_connections(listener, subs_by_topic)?;

    // Ok(())
    Ok(())
}
