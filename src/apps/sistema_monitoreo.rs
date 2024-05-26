extern crate gio;
extern crate gtk;
use gio::prelude::*;
use gtk::prelude::*;
//use rustx::apps::camera::Camera;
use rustx::mqtt_client::MQTTClient;
use std::env::args;
use std::error::Error;
use std::net::SocketAddr;
use std::thread;


/// Lee el IP del cliente y el puerto en el que el cliente se va a conectar al servidor.
fn load_port() -> Result<(String, u16), Box<dyn Error>> {
    let argv = std::env::args().collect::<Vec<String>>();
    if argv.len() != 3 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Cantidad de argumentos inválido. Debe ingresar la dirección IP y 
            el puerto en el que desea correr el servidor.",
        )));
    }
    let ip = &argv[1];
    let port = match argv[2].parse::<u16>() {
        Ok(port) => port,
        Err(_) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "El puerto proporcionado no es válido",
            )))
        }
    };

    Ok((ip.to_string(), port))
}

fn establish_mqtt_broker_connection(
    broker_addr: &SocketAddr,
) -> Result<MQTTClient, Box<dyn std::error::Error>> {
    let mqtt_client_res = MQTTClient::connect_to_broker(broker_addr);
    match mqtt_client_res {
        Ok(mqtt_client) => {
            println!("Cliente: Conectado al broker MQTT.");
            Ok(mqtt_client)
        }
        Err(e) => {
            println!("Sistema-Camara: Error al conectar al broker MQTT: {:?}", e);
            Err(e.into())
        }
    }
}

fn subscribe_to_topics(mut mqtt_client: MQTTClient) {
    let res_sub = mqtt_client.mqtt_subscribe(1, vec![(String::from("Cam"))]);
    match res_sub {
        Ok(_) => println!("Cliente: Hecho un subscribe exitosamente"),
        Err(e) => println!("Cliente: Error al hacer un subscribe: {:?}", e),
    }

    // Que lea del topic al/os cual/es hizo subscribe, implementando [].
    let h = thread::spawn(move || {
        while let Ok(msg) = mqtt_client.mqtt_receive_msg_from_subs_topic() {
            println!("Cliente: Recibo estos msg_bytes: {:?}", msg);
            // Acá ya se podría hacer algo como lo de abajo, pero no descomentarlo xq rompe, hay que revisar
            //let camera_recibida = Camera::from_bytes(&msg.get_payload());
            //println!("Cliente: Recibo cámara: {:?}", camera_recibida);
        }

        // Cliente termina de utilizar mqtt
        mqtt_client.finalizar();
    });

    if h.join().is_err() {
        println!("Cliente: error al esperar a hijo que recibe msjs");
    }
}

fn main() {
    let res = load_port();
    let (ip, port) = match res {
        Ok((ip, port)) => (ip, port),
        Err(e) => {
            println!("Error al cargar el puerto: {:?}", e);
            return;
        }
    };

    let broker_addr: String = format!("{}:{}", ip, port);
    let broker_addr = broker_addr
        .parse::<SocketAddr>()
        .expect("Dirección no válida");

    let hijo_connect = thread::spawn(move || {
        let mqtt_client_res = establish_mqtt_broker_connection(&broker_addr);
        match mqtt_client_res {
            Ok(mqtt_client) => subscribe_to_topics(mqtt_client),
            Err(e) => println!(
                "Error al establecer la conexión con el broker MQTT: {:?}",
                e
            ),
        }
    });

    let hijo_ui = thread::spawn(move || {
        let application = gtk::Application::new(
            Some("fi.uba.sistemamonitoreo"),
            gio::ApplicationFlags::FLAGS_NONE,
        )
        .expect("Fallo en iniciar la aplicacion");
        application.connect_activate(build_ui);
        application.run(&[]);
    });

    if hijo_connect.join().is_err() {
        println!("Error al esperar a la conexión y suscripción.");
    }
    if hijo_ui.join().is_err() {
        println!("Error al esperar a la ui.");
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
