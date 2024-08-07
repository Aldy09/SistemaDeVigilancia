use std::collections::HashMap;
use std::str::{from_utf8, Utf8Error};
use std::time::{Duration, Instant};

use crate::apps::apps_mqtt_topics::AppsMqttTopics;
use crate::apps::incident_data::incident_state::IncidentState;
use crate::apps::incident_data::{
    incident::Incident, incident_info::IncidentInfo, incident_source::IncidentSource,
};
use crate::apps::place_type::PlaceType;
use crate::apps::sist_camaras::camera_state::CameraState;
use crate::apps::sist_dron::dron_current_info::DronCurrentInfo;
use crate::apps::sist_dron::dron_state::DronState;
use crate::mqtt::messages::publish_message::PublishMessage;

use crate::apps::sist_camaras::camera::Camera;
use crate::apps::vendor::{
    HttpOptions, Map, MapMemory, Place, Places, Position, Style, Tiles, TilesManager,
};
use crate::apps::{places, plugins::ImagesPluginData};
use crate::mqtt::mqtt_utils::will_message_utils::app_type::AppType;
use crate::mqtt::mqtt_utils::will_message_utils::will_content::WillContent;
use crossbeam_channel::{unbounded, Receiver as CrossbeamReceiver, Sender as CrossbeamSender};
use egui::Color32;
use egui::Context;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    OpenStreetMap,
    Geoportal,
    MapboxStreets,
    MapboxSatellite,
    LocalTiles,
}

fn http_options() -> HttpOptions {
    HttpOptions {
        cache: None,
        /*cache: if std::env::var("NO_HTTP_CACHE").is_ok() {
            None
        } else {
            Some(".cache".into())
        },*/
        ..Default::default()
    }
}

fn providers(egui_ctx: Context) -> HashMap<Provider, Box<dyn TilesManager + Send>> {
    let mut providers: HashMap<Provider, Box<dyn TilesManager + Send>> = HashMap::default();

    providers.insert(
        Provider::OpenStreetMap,
        Box::new(Tiles::with_options(
            super::super::vendor::sources::OpenStreetMap,
            http_options(),
            egui_ctx.to_owned(),
        )),
    );

    providers.insert(
        Provider::Geoportal,
        Box::new(Tiles::with_options(
            super::super::vendor::sources::Geoportal,
            http_options(),
            egui_ctx.to_owned(),
        )),
    );

    providers.insert(
        Provider::LocalTiles,
        Box::new(super::super::local_tiles::LocalTiles::new(
            egui_ctx.to_owned(),
        )),
    );

    // Pass in a mapbox access token at compile time. May or may not be what you want to do,
    // potentially loading it from application settings instead.
    let mapbox_access_token = std::option_env!("MAPBOX_ACCESS_TOKEN");

    // We only show the mapbox map if we have an access token
    if let Some(token) = mapbox_access_token {
        providers.insert(
            Provider::MapboxStreets,
            Box::new(Tiles::with_options(
                super::super::vendor::sources::Mapbox {
                    style: super::super::vendor::sources::MapboxStyle::Streets,
                    access_token: token.to_string(),
                    high_resolution: false,
                },
                http_options(),
                egui_ctx.to_owned(),
            )),
        );
        providers.insert(
            Provider::MapboxSatellite,
            Box::new(Tiles::with_options(
                super::super::vendor::sources::Mapbox {
                    style: super::super::vendor::sources::MapboxStyle::Satellite,
                    access_token: token.to_string(),
                    high_resolution: true,
                },
                http_options(),
                egui_ctx.to_owned(),
            )),
        );
    }

    providers
}

#[derive(Debug)]
struct IncidentWithDrones {
    incident_info: IncidentInfo,
    drones: Vec<DronCurrentInfo>,
}

pub struct UISistemaMonitoreo {
    providers: HashMap<Provider, Box<dyn TilesManager + Send>>,
    selected_provider: Provider,
    map_memory: MapMemory,
    images_plugin_data: ImagesPluginData,
    click_watcher: super::super::plugins::ClickWatcher,
    incident_dialog_open: bool,
    latitude: String,
    longitude: String,
    publish_incident_tx: Sender<Incident>,
    publish_message_rx: CrossbeamReceiver<PublishMessage>,
    places: Places,
    last_incident_id: u8,
    exit_tx: Sender<bool>,
    incidents_to_resolve: Vec<IncidentWithDrones>, // posicion 0  --> (inc_id_to_resolve, drones(dron1, dron2)) // posicion 1 --> (inc_id_to_resolve 2, drones(dron1, dron2))
    hashmap_incidents: HashMap<IncidentInfo, Incident>, //
    error_tx: CrossbeamSender<String>,
    error_rx: CrossbeamReceiver<String>,
    error_message: Option<String>,
    error_display_start: Option<Instant>,
}

impl UISistemaMonitoreo {
    pub fn new(
        egui_ctx: Context,
        tx: Sender<Incident>,
        publish_message_rx: CrossbeamReceiver<PublishMessage>,
        exit_tx: Sender<bool>,
    ) -> Self {
        egui_extras::install_image_loaders(&egui_ctx);

        let images_plugin_data = ImagesPluginData::new(egui_ctx.to_owned());
        let places = Self::initialize_places();
        let (error_tx, error_rx) = unbounded();

        Self {
            providers: providers(egui_ctx.to_owned()),
            selected_provider: Provider::OpenStreetMap,
            map_memory: MapMemory::default(),
            images_plugin_data,
            click_watcher: Default::default(),
            incident_dialog_open: false,
            latitude: String::new(),
            longitude: String::new(),
            publish_incident_tx: tx,
            publish_message_rx,
            places,
            last_incident_id: 0,
            exit_tx,
            incidents_to_resolve: Vec::new(),
            hashmap_incidents: HashMap::new(),
            error_tx,
            error_rx,
            error_message: None,
            error_display_start: None,
        }
    }

    fn create_style_with_color(r: u8, g: u8, b: u8) -> Style {
        Style {
            symbol_color: Color32::from_rgb(r, g, b),
            ..Default::default()
        }
    }

    fn initialize_places() -> Places {
        let mantainance_style = Self::create_style_with_color(255, 165, 0); // Color naranja
        let mantainance_ui = Self::create_maintenance_place(mantainance_style);
        let mut places = Places::new();
        places.add_place(mantainance_ui);
        places
    }

    fn create_maintenance_place(style: Style) -> Place {
        Place {
            position: places::mantenimiento(),
            label: "Mantenimiento".to_string(),
            symbol: '🔋',
            style,
            id: 0,
            place_type: PlaceType::Mantainance,
        }
    }

    /// Envía internamente a otro hilo el `incident` recibido, para publicarlo por mqtt.
    fn send_incident_for_publish(&self, incident: Incident) {
        println!("Enviando incidente: {:?}", incident);
        let _ = self.publish_incident_tx.send(incident);
    }

    fn create_camera_style(camera_state: CameraState) -> Style {
        match camera_state {
            CameraState::Active => Style {
                symbol_color: Color32::from_rgb(0, 255, 0), // Color verde
                ..Default::default()
            },
            CameraState::SavingMode => Style::default(),
        }
    }

    fn create_camera_place(camera: &Camera, style: Style) -> Place {
        let camera_id = camera.get_id();
        let (latitude, longitude) = (camera.get_latitude(), camera.get_longitude());

        Place {
            position: Position::from_lon_lat(longitude, latitude),
            label: format!("Camera {}", camera_id),
            symbol: '📷',
            style,
            id: camera_id,
            place_type: PlaceType::Camera,
        }
    }

    fn update_camera_on_map(&mut self, camera: Camera) {
        let camera_id = camera.get_id();

        if camera.is_not_deleted() {
            self.places.remove_place(camera_id, PlaceType::Camera);

            let style = Self::create_camera_style(camera.get_state());
            let camera_ui = Self::create_camera_place(&camera, style);
            self.places.add_place(camera_ui);
        } else {
            self.places.remove_place(camera_id, PlaceType::Camera);
        }
    }

    /// Se encarga de procesar y agregar o eliminar una cámara recibida al mapa.
    fn handle_camera_message(&mut self, publish_message: PublishMessage) {
        let camera = Camera::from_bytes(&publish_message.get_payload());
        println!(
            "UI: recibida cámara: {:?}, estado: {:?}",
            camera,
            camera.get_state()
        );

        self.update_camera_on_map(camera);
    }

    /// Se encarga de procesar y agregar un dron recibido al mapa.
    fn handle_drone_message(&mut self, msg: PublishMessage) {
        if let Ok(dron) = DronCurrentInfo::from_bytes(msg.get_payload()) {
            /*println!(
                "UI: recibido dron: {:?}, estado: {:?}",
                dron,
                dron.get_state()
            );*/
            // Si ya existía el dron, se lo elimina, porque que me llegue nuevamente significa que se está moviendo.
            let dron_id = dron.get_id();
            self.places.remove_place(dron_id, PlaceType::Dron);

            if dron.get_state() == DronState::ManagingIncident {
                // Llegó a la posición del inc.
                if let Some(inc_info) = dron.get_inc_id_to_resolve() {
                    // Busca el incidente en el vector.
                    let incident_index = self
                        .incidents_to_resolve
                        .iter()
                        .position(|incident| incident.incident_info == inc_info);
                    //.position(|incident| incident.incident_info.get_inc_id() == inc_id); // <--pre refactor decía esto

                    match incident_index {
                        Some(index) => {
                            // Si el incidente ya existe, agrega el dron al vector de drones del incidente.
                            self.incidents_to_resolve[index].drones.push(dron.clone());
                        }
                        None => {
                            // Si no tengo guardado el inc_id_to_res, crea una nueva posicion con el dron respectivo.
                            self.incidents_to_resolve.push(IncidentWithDrones {
                                incident_info: inc_info,
                                drones: vec![dron.clone()],
                            });
                        }
                    }
                }
            }

            for incident in self.incidents_to_resolve.iter() {
                if incident.drones.len() == 2 {
                    let inc_info = &incident.incident_info;
                    if let Some(mut incident) = self.hashmap_incidents.remove(inc_info) {
                        incident.set_resolved();
                        // Obtengo el source del incidente, para pasarle un place_type acorde al remove_place
                        // y lo remuevo de la lista de places a mostrar en el mapa.
                        let place_type = PlaceType::from_inc_source(incident.get_source());
                        self.places.remove_place(inc_info.get_inc_id(), place_type);

                        self.send_incident_for_publish(incident);
                    }
                }
            }

            // Crea lo necesario para dibujar al dron
            let (lat, lon) = dron.get_current_position();
            let dron_pos = Position::from_lon_lat(lon, lat);

            // Se crea el label a mostrar por pantalla, según si está o no volando.
            let dron_label;
            if let Some((dir, speed)) = dron.get_flying_info() {
                let (dir_lat, dir_lon) = dir;
                // El dron está volando.
                dron_label = format!(
                    "Dron {}\n   dir: ({:.2}, {:.2})\n   vel: {} km/h",
                    dron_id, dir_lat, dir_lon, speed
                );
            } else {
                dron_label = format!("Dron {}", dron_id);
            }

            // Se crea el place y se lo agrega al mapa.
            let dron_ui = Place {
                position: dron_pos,
                label: dron_label,
                symbol: '🚁',
                style: Style::default(),
                id: dron.get_id(),
                place_type: PlaceType::Dron, // Para luego buscarlo en el places.
            };

            self.places.add_place(dron_ui);
        }
        //let _ = self.repaint_tx.send(true);
        //let _ = self.repaint_tx.send(true);
    }

    /// Recibe un PublishMessage de topic Inc, y procesa el incidente recibido
    /// (se lo guarda para continuar procesándolo, y lo muestra en la ui).
    fn handle_incident_message(&mut self, msg: PublishMessage) {
        if let Ok(inc) = Incident::from_bytes(msg.get_payload()) {
            // Agregamos el incidente (add_incident) solamente si él no fue creado por sist monitoreo.
            if *inc.get_source() == IncidentSource::Automated
                && *inc.get_state() == IncidentState::ActiveIncident
            {
                self.add_incident(&inc);
            }
        }
    }

    /// Crea el Place para el incidente recibido, lo agrega a la ui para que se muestre por pantalla,
    /// y lo agrega a un hashmap para continuar procesándolo (Aux: rever tema ids que quizás se pisen cuando camaras publiquen incs).
    fn add_incident(&mut self, incident: &Incident) {
        let custom_style = Self::create_style_with_color(255, 0, 0); // Color rojo
        let new_place_incident = self.create_place_for_incident(incident, &custom_style);
        self.places.add_place(new_place_incident);
        self.store_incident_info(incident);
    }

    fn create_place_for_incident(&self, incident: &Incident, custom_style: &Style) -> Place {
        let place_type = PlaceType::from_inc_source(incident.get_source());
        let (lat, lon) = incident.get_position();
        Place {
            position: Position::from_lon_lat(lon, lat),
            label: format!("Incident {}", incident.get_id()),
            symbol: '⚠',
            style: custom_style.clone(),
            id: incident.get_id(),
            place_type,
        }
    }

    fn store_incident_info(&mut self, incident: &Incident) {
        let inc_info = IncidentInfo::new(incident.get_id(), *incident.get_source());
        let inc_to_store = incident.clone();
        self.hashmap_incidents.insert(inc_info, inc_to_store);
    }

    fn get_next_incident_id(&mut self) -> u8 {
        self.last_incident_id += 1;
        self.last_incident_id
    }

    fn handle_disconnection_message(
        &mut self,
        publish_message: PublishMessage,
    ) -> Result<(), Utf8Error> {
        let will_content_res = WillContent::will_content_from_string(from_utf8(&publish_message.get_payload())?);
        
        if let Ok(will_content) = will_content_res {
            self.process_will_content(will_content)?;
        }

        Ok(())
    }

    fn process_will_content(&mut self, will_content: WillContent) -> Result<(), Utf8Error> {
        let app_type = will_content.get_app_type_identifier();
        let id_option = will_content.get_id(); // es un option porque solo dron tiene id en este contexto.
        let place_type = PlaceType::from_app_type_will_content(&app_type);

        match app_type {
            AppType::Cameras => self.handle_camera_disconnection(place_type),
            AppType::Dron => self.handle_drone_disconnection(id_option, place_type),
            AppType::Monitoreo => {},
        }
        Ok(())
    }

    fn handle_camera_disconnection(&mut self, place_type: PlaceType) {
        // Se eliminan Todas las cámaras
        self.places.remove_places(place_type)
    }

    fn handle_drone_disconnection(&mut self, id_option: Option<u8>, place_type: PlaceType) {
        if let Some(id) = id_option {
            // Se elimina el dron de id indicado, porque el mismo se desconectó.
            self.places.remove_place(id, place_type)
        }
    }

    fn handle_mqtt_messages(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |_ui| {
            if let Ok(publish_message) = self.publish_message_rx.try_recv() {
                self.route_message(publish_message);
            }
        });
    }

    fn route_message(&mut self, publish_message: PublishMessage) {
        let topic_str = publish_message.get_topic_name();
        if let Ok(topic) = AppsMqttTopics::topic_from_str(&topic_str) {
            match topic {
                AppsMqttTopics::CameraTopic => {
                    self.handle_camera_message(publish_message)
                },
                AppsMqttTopics::DronTopic => {
                    self.handle_drone_message(publish_message)
                },
                AppsMqttTopics::IncidentTopic => {
                    self.handle_incident_message(publish_message)
                },
                AppsMqttTopics::DescTopic => {
                    println!("Recibido mensaje de desconexión.");
                    let _ = self.handle_disconnection_message(publish_message);
                },
            }
        }
    }

    fn setup_map(&mut self, ctx: &egui::Context) {
        let rimless = egui::Frame {
            fill: ctx.style().visuals.panel_fill,
            ..Default::default()
        };

        egui::CentralPanel::default()
            .frame(rimless)
            .show(ctx, |ui| {
                let my_position = places::obelisco();
                let tiles = self
                    .providers
                    .get_mut(&self.selected_provider)
                    .unwrap()
                    .as_mut();
                let map = Map::new(Some(tiles), &mut self.map_memory, my_position)
                    .with_plugin(self.places.clone())
                    .with_plugin(super::super::plugins::images(&mut self.images_plugin_data))
                    .with_plugin(super::super::plugins::CustomShapes {})
                    .with_plugin(&mut self.click_watcher);

                ui.add(map);
                self.setup_map_controls(ui);
            });
    }

    fn setup_map_controls(&mut self, ui: &mut egui::Ui) {
        use super::super::windows::*;
        zoom(ui, &mut self.map_memory);
        go_to_my_position(ui, &mut self.map_memory);
        self.click_watcher.show_position(ui);
        controls(
            ui,
            &mut self.selected_provider,
            &mut self.providers.keys(),
            &mut self.images_plugin_data,
        );
    }

    fn setup_top_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.incident_menu(ui);
                self.exit_menu(ui, ctx);
            });
        });
    }

    fn incident_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Incidente", |ui| {
            if !self.incident_dialog_open && ui.button("Alta Incidente").clicked() {
                self.incident_dialog_open = true;
            }
            if self.incident_dialog_open {
                self.incident_dialog(ui);
            }
        });
    }

    fn incident_dialog(&mut self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            self.incident_position_inputs(ui);
            if ui.button("OK").clicked() {
                self.process_incident();
            }
        });
    }

    fn incident_position_inputs(&mut self, ui: &mut egui::Ui) {
        ui.label("Latitud:");
        let _latitude_input = ui.add_sized(
            [100.0, 20.0],
            egui::TextEdit::singleline(&mut self.latitude),
        );
        ui.label("Longitud:");
        let _longitude_input = ui.add_sized(
            [100.0, 20.0],
            egui::TextEdit::singleline(&mut self.longitude),
        );
    }

    fn process_incident(&mut self) {
        match self.parse_location() {
            Ok(location) => self.handle_successful_parse(location),
            Err(err) => self.send_error_message(err),
        }
    }

    fn parse_location(&self) -> Result<(f64, f64), &'static str> {
        let latitude_result = self.latitude.to_string().parse::<f64>();
        let longitude_result = self.longitude.to_string().parse::<f64>();

        match (latitude_result, longitude_result) {
            (Ok(latitude), Ok(longitude)) => Ok((latitude, longitude)),
            (Err(_), _) => Err("Latitud ingresada incorrectamente. Por favor, intente de nuevo."),
            (_, Err(_)) => Err("Longitud ingresada incorrectamente. Por favor, intente de nuevo."),
        }
    }

    fn handle_successful_parse(&mut self, location: (f64, f64)) {
        let incident = Incident::new(
            self.get_next_incident_id(),
            location,
            IncidentSource::Manual,
        );
        self.add_incident(&incident);
        self.send_incident_for_publish(incident);
        self.incident_dialog_open = false;
    }

    fn send_error_message(&self, error_message: &'static str) {
        match self.error_tx.send(error_message.to_string()) {
            Ok(_) => println!("Mensaje de error enviado correctamente."),
            Err(_) => println!("Error al enviar mensaje de error."),
        }
    }

    fn exit_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if ui.button("Salir").clicked() {
            self.exit(ctx);
        }
    }

    fn exit(&self, ctx: &egui::Context) {
        if let Err(e) = self.exit_tx.send(true) {                
            println!("Error al intentar salir: {:?}", e);
            return;
        }
        println!("Iniciando proceso para salir");
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn request_repaint_after(&mut self, milliseconds: u64, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |_ui| {
            ctx.request_repaint_after(std::time::Duration::from_millis(milliseconds));
        });
    }
    
    fn draw_ui(&mut self, ui: &mut egui::Ui) {
        self.check_for_errors();
        let error_msg = &self.error_message.clone();
        if let Some(error) = error_msg {
            self.display_error_window(ui, error);
        }
    }
    
    fn check_for_errors(&mut self) {
        if let Ok(error) = self.error_rx.try_recv() {
            self.error_message = Some(error);
            self.error_display_start = Some(Instant::now());
        }
    }

    fn display_error_window(&mut self, ui: &mut egui::Ui, error: &String) {
        if self.error_display_start.unwrap().elapsed() < Duration::from_secs(5) {
            let screen_size = ui.ctx().screen_rect().size();
            let window_size = egui::vec2(200.0, 200.0);
            let pos = self.calculate_center_position(screen_size, window_size);

            egui::Window::new("Error")
                .collapsible(false)
                .title_bar(true)
                .fixed_pos(pos)
                .min_size(window_size)
                .show(ui.ctx(), |ui| {
                    ui.heading(error);
                });
        } else {
            self.error_message = None;
            self.error_display_start = None;
        }
    }

    fn calculate_center_position(&mut self, screen_size: egui::Vec2, window_size: egui::Vec2) -> egui::Pos2 {
        egui::pos2(
            (screen_size.x - window_size.x) / 2.0,
            (screen_size.y - window_size.y) / 2.0,
        )
    }

    fn draw_ui_wrapper(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_ui(ui);
        });
    }
}

impl eframe::App for UISistemaMonitoreo {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.request_repaint_after(150, ctx);
        self.draw_ui_wrapper(ctx);
        self.handle_mqtt_messages(ctx);
        self.setup_map(ctx);
        self.setup_top_menu(ctx);
    }
}
