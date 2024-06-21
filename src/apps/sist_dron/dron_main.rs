use std::io::Error;

use rustx::apps::sist_dron::{dron::Dron, utils::get_id_and_broker_address};

fn main() -> Result<(), Error> {
    let (id, broker_addr) = get_id_and_broker_address()?;

    let dron = Dron::new(id, broker_addr);
    println!("Probando, dron {:?}", dron);

    Ok(())
}
