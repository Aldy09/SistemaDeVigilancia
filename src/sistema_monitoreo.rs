extern crate gtk;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Button, Entry};

fn build_ui(application: &gtk::Application) {
    // Crea una ventana
    let window = ApplicationWindow::new(application);
    window.set_title("Formulario GTK en Rust");
    window.set_default_size(400, 200);

    // Crea una caja vertical para organizar los elementos
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
    window.add(&vbox);

    // Crea una entrada de texto
    let entry = Entry::new();
    entry.set_text("Ingrese su nombre");
    vbox.add(&entry);

    // Crea un botón
    let button = Button::with_label("Enviar");
    vbox.add(&button);

    // Conecta el evento "clicked" del botón
    button.connect_clicked(move |_| {
        println!("Texto ingresado: {}", entry.text());
        entry.set_text(""); // Limpia la entrada después de presionar el botón
    });

    // Muestra todos los elementos
    window.show_all();
}

fn main() {
    // Inicializa la aplicación GTK
    let application = Application::builder()
        .application_id("com.example.myapp")
        .build();

    // Conecta la función build_ui al evento "activate" de la aplicación
    application.connect_activate(|app| {
        build_ui(app);
    });

    // Ejecuta la aplicación
    application.run();
}
