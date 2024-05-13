extern crate gio;
extern crate gtk;

use gio::prelude::*;
use gtk::prelude::*;

fn main() {
    let application = gtk::Application::new(
        Some("fi.uba.sistemamonitoreo"),
        gio::ApplicationFlags::FLAGS_NONE,
    )
    .expect("Fallo en iniciar la aplicacion");

    application.connect_activate(build_ui);

    application.run(&[]);
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);
    window.set_title("Sistema de Monitoreo");
    window.set_default_size(800, 600);

    // Crear un menú _ Código de creación del menú
    let menubar = gtk::MenuBar::new();

    let menu_incidents = gtk::Menu::new();
    let item_add = gtk::MenuItem::with_label("Alta de Incidente");
    let item_edit = gtk::MenuItem::with_label("Modificación de Incidente");
    let item_delete = gtk::MenuItem::with_label("Baja de Incidente");

    menu_incidents.append(&item_add);
    menu_incidents.append(&item_edit);
    menu_incidents.append(&item_delete);

    item_add.connect_activate(move |_| {
        show_add_form();
    });

    item_edit.connect_activate(|_| {
        println!("Modificación de Incidente");
    });

    item_delete.connect_activate(|_| {
        println!("Baja de Incidente");
    });

    let item_quit = gtk::MenuItem::with_label("Salir");
    item_quit.connect_activate(|_| {
        gtk::main_quit();
    });

    let menu_app = gtk::Menu::new();
    menu_app.append(&item_quit);

    let item_incidents = gtk::MenuItem::with_label("Incidentes");
    let item_exit = gtk::MenuItem::with_label("Salir");

    item_incidents.set_submenu(Some(&menu_incidents));
    item_exit.connect_activate(|_| {
        gtk::main_quit();
    });

    menubar.append(&item_incidents);
    menubar.append(&item_exit);

    // Crear un contenedor Box para contener el menú y el mapa
    let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    // Agregar el menú al contenedor
    layout.pack_start(&menubar, false, false, 0);

    // Crear un Image para mostrar el mapa
    let map_image = gtk::Image::from_file("mapa_temp.png");
    // Agregar el mapa al contenedor
    layout.pack_start(&map_image, true, true, 0);

    // Crear un Overlay para superponer la imagen
    let overlay = gtk::Overlay::new();
    // Agregar el Overlay al contenedor
    layout.pack_start(&overlay, true, true, 0);

    // Crear un Image para la imagen superpuesta
    let superimposed_image = gtk::Image::from_file("dron.png");

    // Agregar la imagen superpuesta al Overlay
    overlay.add(&superimposed_image);
    
    window.add(&layout);
    window.show_all();
}

fn show_add_form() {
    let dialog = gtk::Dialog::with_buttons(
        Some("Alta de Incidente"),
        None::<&gtk::Window>,
        gtk::DialogFlags::MODAL,
        &[
            ("Ok", gtk::ResponseType::Ok),
            ("Cancel", gtk::ResponseType::Cancel),
        ],
    );

    let content_area = dialog.get_content_area();
    content_area.set_spacing(5);

    let name_label = gtk::Label::new(Some("Nombre del Incidente:"));
    let name_entry = gtk::Entry::new();
    content_area.add(&name_label);
    content_area.add(&name_entry);

    let x_label = gtk::Label::new(Some("Coordenada X:"));
    let x_entry = gtk::Entry::new();
    content_area.add(&x_label);
    content_area.add(&x_entry);

    let y_label = gtk::Label::new(Some("Coordenada Y:"));
    let y_entry = gtk::Entry::new();
    content_area.add(&y_label);
    content_area.add(&y_entry);

    dialog.show_all();

    dialog.connect_response(move |dialog, response_type| {
        if response_type == gtk::ResponseType::Ok {
            let name = name_entry.get_text().to_string();
            let x_coord = x_entry.get_text().to_string();
            let y_coord = y_entry.get_text().to_string();

            println!("Nombre del Incidente: {}", name);
            println!("Coordenada X: {}", x_coord);
            println!("Coordenada Y: {}", y_coord);
        }

        dialog.close();
    });
}
