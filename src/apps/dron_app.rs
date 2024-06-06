//use rustx::apps::dron::Dron;
//use crate::apps::dron::Dron;

use crate::apps::dron::Dron; // <-- compila, pero si descomento el bin dron en el toml ya no compila (?).
                             //use rustx::apps::dron::Dron; // (<-- este no le gusta, ni con ni sin estar en el toml)

#[allow(dead_code)]
/// Aplicación que ejecutará cada dron.
/// Este archivo es el que figura en el Cargo.toml para cada dron.
fn main() {
    let dron = Dron::new(1);
    println!("Creado el dron: {:?}", dron);
}
