use std::collections::HashMap;

use crate::apps::incident::Incident;
use crate::messages::publish_message::PublishMessage;

use super::camera::Camera;
use super::places;
use super::plugins::ImagesPluginData;
use super::vendor::{
    HttpOptions, Map, MapMemory, Place, Places, Position, Style, Tiles, TilesManager,
};
use crossbeam::channel::Receiver;
use egui::menu;
use egui::Context;
//use tokio::net::unix::pipe::Receiver;
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
            super::vendor::sources::OpenStreetMap,
            http_options(),
            egui_ctx.to_owned(),
        )),
    );

    providers.insert(
        Provider::Geoportal,
        Box::new(Tiles::with_options(
            super::vendor::sources::Geoportal,
            http_options(),
            egui_ctx.to_owned(),
        )),
    );

    providers.insert(
        Provider::LocalTiles,
        Box::new(super::local_tiles::LocalTiles::new(egui_ctx.to_owned())),
    );

    // Pass in a mapbox access token at compile time. May or may not be what you want to do,
    // potentially loading it from application settings instead.
    let mapbox_access_token = std::option_env!("MAPBOX_ACCESS_TOKEN");

    // We only show the mapbox map if we have an access token
    if let Some(token) = mapbox_access_token {
        providers.insert(
            Provider::MapboxStreets,
            Box::new(Tiles::with_options(
                super::vendor::sources::Mapbox {
                    style: super::vendor::sources::MapboxStyle::Streets,
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
                super::vendor::sources::Mapbox {
                    style: super::vendor::sources::MapboxStyle::Satellite,
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

pub struct UISistemaMonitoreo {
    providers: HashMap<Provider, Box<dyn TilesManager + Send>>,
    selected_provider: Provider,
    map_memory: MapMemory,
    images_plugin_data: ImagesPluginData,
    click_watcher: super::plugins::ClickWatcher,
    incident_dialog_open: bool,
    latitude: String,
    longitude: String,
    publish_incident_tx: Sender<Incident>,
    publish_message_rx: Receiver<PublishMessage>,
    places: Places,
    last_incident_id: u8,
    exit_tx: Sender<bool>,
}

impl UISistemaMonitoreo {
    pub fn new(
        egui_ctx: Context,
        tx: Sender<Incident>,
        publish_message_rx: Receiver<PublishMessage>,
        exit_tx: Sender<bool>,
    ) -> Self {
        egui_extras::install_image_loaders(&egui_ctx);

        // Data for the `images` plugin showcase.
        let images_plugin_data = ImagesPluginData::new(egui_ctx.to_owned());
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
            places: super::vendor::Places::new(),
            last_incident_id: 0,
            exit_tx,
        }
    }
    fn send_incident(&self, incident: Incident) {
        println!("Enviando incidente: {:?}", incident);
        let _ = self.publish_incident_tx.send(incident);
    }
    fn handle_camera_message(&mut self, publish_message: PublishMessage) {
        let camera = Camera::from_bytes(&publish_message.get_payload());
        if camera.is_not_deleted() {
            let (latitude, longitude) = (camera.get_latitude(), camera.get_longitude());
            let camera_id = camera.get_id();
            let new_place = Place {
                position: Position::from_lon_lat(longitude, latitude),
                label: format!("Camera {}", camera_id),
                symbol: '📷',
                style: Style::default(),
                id: camera_id,
                place_type: "Camera".to_string(),
            };
            self.places.add_place(new_place);
        } else {
            self.places.remove_place(camera.get_id(), "Camera".to_string());
        }
    }

    fn handle_drone_message(&mut self, _publish_message: PublishMessage) {
        //cosas del drone
    }

    pub fn get_next_incident_id(&mut self) -> u8 {
        self.last_incident_id += 1;
        self.last_incident_id
    }
}

impl eframe::App for UISistemaMonitoreo {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let rimless = egui::Frame {
            fill: ctx.style().visuals.panel_fill,
            ..Default::default()
        };

        egui::CentralPanel::default().show(ctx, |_ui| {
            if let Ok(publish_message) = self.publish_message_rx.try_recv() {
                if publish_message.get_topic_name() == "Cam" {
                    self.handle_camera_message(publish_message);
                } else if publish_message.get_topic_name() == "Drone" {
                    self.handle_drone_message(publish_message);
                }
            }
        });

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
                    .with_plugin(super::plugins::images(&mut self.images_plugin_data))
                    .with_plugin(super::plugins::CustomShapes {})
                    .with_plugin(&mut self.click_watcher);

                ui.add(map);

                {
                    use super::windows::*;
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

                egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
                    egui::menu::bar(ui, |ui| {
                        menu::bar(ui, |ui| {
                            ui.menu_button("Incidente", |ui| {
                                if !self.incident_dialog_open
                                    && ui.button("Alta Incidente").clicked()
                                {
                                    self.incident_dialog_open = true;
                                }
                                if self.incident_dialog_open {
                                    ui.add_space(5.0);
                                    ui.horizontal(|ui| {
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

                                        if ui.button("OK").clicked() {
                                            let latitude_text = self.latitude.to_string();
                                            let longitude_text = self.longitude.to_string();

                                            println!("Latitud: {}", latitude_text);
                                            println!("Longitud: {}", longitude_text);

                                            let latitude = latitude_text.parse::<f64>().unwrap();
                                            let longitude: f64 =
                                                longitude_text.parse::<f64>().unwrap();
                                            let incident = Incident::new(
                                                self.get_next_incident_id(),
                                                latitude,
                                                longitude,
                                            );
                                            let new_place_incident = Place {
                                                position: Position::from_lon_lat(longitude, latitude),
                                                label: format!("Incident {}", incident.get_id()),
                                                symbol: '⚠',
                                                style: Style::default(),
                                                id: incident.get_id(),
                                                place_type: "Incident".to_string(),
                                            };
                                            self.places.add_place(new_place_incident);
                                            self.send_incident(incident);
                                            self.incident_dialog_open = false;
                                        }
                                    });
                                }
                            });
                            if ui.button("Salir").clicked() {
                                // Indicar que se desea salir
                                match self.exit_tx.send(true) {
                                    Ok(_) => println!("Iniciando proceso para salir"),
                                    Err(_) => println!("Error al intentar salir"),
                                }
                                
                                
                            }
                        });
                    });
                });
            });
    }
}
