extern crate gio;
extern crate gtk;

use std::thread;

use gio::prelude::*;
use gtk::prelude::*;
use rustx::{connect_message::ConnectMessage, mqtt_client::MQTTClient};

fn main() {
    let application = gtk::Application::new(
        Some("fi.uba.sistemamonitoreo"),
        gio::ApplicationFlags::FLAGS_NONE,
    )
    .expect("Fallo en iniciar la aplicacion");

    application.connect_activate(build_ui);

    application.run(&[]);
}

fn connect_and_subscribe() {
    let ip = "127.0.0.1".to_string();
    let port = 9090;
    let broker_addr = format!("{}:{}", ip, port)
        .parse()
        .expect("Dirección no válida");
    
    // Cliente usa funciones connect, publish, y subscribe de la lib.
    let mqtt_client_res = MQTTClient::connect_to_broker(&broker_addr);
    match mqtt_client_res {
        Ok(mut mqtt_client) => {
            //info!("Conectado al broker MQTT."); //
            println!("Cliente: Conectado al broker MQTT.");

            let mut mqtt_client_para_hijo = mqtt_client.mqtt_clone();

            let h_sub = thread::spawn(move || {
                // Cliente usa subscribe // Aux: me suscribo al mismo topic al cual el otro hilo está publicando, para probar
                let res_sub =
                    mqtt_client_para_hijo.mqtt_subscribe(1, vec![(String::from("Cam"), 1)]);
                match res_sub {
                    Ok(_) => {
                        println!("Cliente: Hecho un subscribe exitosamente");
                    }
                    Err(e) => {
                        println!("Cliente: Error al hacer un subscribe: {:?}", e);
                    }
                }
                /*let stream = mqtt_client.get_stream();
                let mut veces = 3;
                while veces > 0 {
                    println!("[loop cliente subscribe] vuelta por intentar leer");
                    // Leo la respuesta
                    let mut bytes_leidos = [0; 6]; // [] Aux temp: 6 para 1 elem, 8 p 2, 10 p 3, en realidad hay que leer el fixed hdr como en server.
                    {
                        match stream.lock(){
                            Ok(mut s) => {let _cant_leida = s.read(&mut bytes_leidos).unwrap();},
                            Err(e) => println!("cliente subscribe: error al lockear: {:?}",e),
                        }
                    }
                    println!("[loop cliente subscribe] vuelta leí bytes: {:?}", bytes_leidos);
                    veces -= 1;
                }*/
            });

            if h_sub.join().is_err() {
                println!("Error al esperar a hijo subscriber.");
            }
        }
        Err(e) => println!("Sistema-Camara: Error al conectar al broker MQTT: {:?}", e),
    }
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);
    window.set_title("Sistema de Monitoreo");
    window.set_default_size(800, 600);

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

    // Un layout para el menubar y la estructura de imágenes superpuestas
    let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    layout.pack_start(&menubar, false, false, 0);

    // Se crea un overlay para permitir superposición de imágenes
    let overlay = gtk::Overlay::new();
    layout.pack_start(&overlay, true, true, 0);

    // Imagen base
    let map_image = gtk::Image::from_file("mapa_temp.png");
    overlay.add(&map_image);

    // Imagen superpuesta: dron
    let superimposed_image = gtk::Image::from_file("dron_70.png");
    overlay.add_overlay(&superimposed_image);
    gtk::WidgetExt::set_halign(&superimposed_image, gtk::Align::Start);
    gtk::WidgetExt::set_valign(&superimposed_image, gtk::Align::Start);
    gtk::WidgetExt::set_margin_start(&superimposed_image, 100);
    gtk::WidgetExt::set_margin_top(&superimposed_image, 100);
    overlay.set_child_index(&superimposed_image, 0);

    let dron2 = gtk::Image::from_file("dron_70.png");
    overlay.add_overlay(&dron2.clone());
    gtk::WidgetExt::set_halign(&dron2, gtk::Align::Start);
    gtk::WidgetExt::set_valign(&dron2, gtk::Align::Start);
    gtk::WidgetExt::set_margin_start(&dron2, 200);
    gtk::WidgetExt::set_margin_top(&dron2, 300);
    overlay.set_child_index(&dron2, 0);

    // Imagen superpuesta: cámara
    let cam_img = gtk::Image::from_file("camara_50.png");
    overlay.add_overlay(&cam_img);
    gtk::WidgetExt::set_halign(&cam_img, gtk::Align::Start);
    gtk::WidgetExt::set_valign(&cam_img, gtk::Align::Start);
    gtk::WidgetExt::set_margin_start(&cam_img, 600);
    gtk::WidgetExt::set_margin_top(&cam_img, 400);
    overlay.set_child_index(&cam_img, 0);

    window.add(&layout);

    let h_connect = thread::spawn(move || {
        connect_and_subscribe();
    });

    if h_connect.join().is_err() {
        println!("Error al esperar a la conexión y suscripción.");
    }

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
