use rustx::apps::sist_dron::dron::Dron;

fn main() {
    let id = 1; // Aux: este id debería leerse de consola #ToDo [].
    let dron = Dron::new(id);
    println!("Probando, dron {:?}", dron);
}
