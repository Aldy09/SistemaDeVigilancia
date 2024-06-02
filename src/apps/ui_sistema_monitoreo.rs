use std::collections::HashMap;

use crate::apps::incident::Incident;

use super::places;
use super::plugins::ImagesPluginData;
use super::vendor::{HttpOptions, Map, MapMemory, Tiles, TilesManager};
use egui::Context;
use egui::menu;
use std::sync::mpsc::{self, Sender};


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
        // Not sure where to put cache on Android, so it will be disabled for now.
        cache: if cfg!(target_os = "android") || std::env::var("NO_HTTP_CACHE").is_ok() {
            None
        } else {
            Some(".cache".into())
        },
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
}

impl UISistemaMonitoreo {
    pub fn new(egui_ctx: Context, tx: Sender<Incident>) -> Self {
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
        }
    }
    fn send_incident(&self,incident: Incident) {
        println!("Enviando incidente: {:?}", incident);
         let _ = self.publish_incident_tx.send(incident);
    }
}

impl eframe::App for UISistemaMonitoreo {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let rimless = egui::Frame {
            fill: ctx.style().visuals.panel_fill,
            ..Default::default()
        };

        egui::CentralPanel::default()
            .frame(rimless)
            .show(ctx, |ui| {
                // Typically this would be a GPS acquired position which is tracked by the map.
                let my_position = places::obelisco();

                let tiles = self
                    .providers
                    .get_mut(&self.selected_provider)
                    .unwrap()
                    .as_mut();
                let attribution = tiles.attribution();

                // In egui, widgets are constructed and consumed in each frame.
                let map = Map::new(Some(tiles), &mut self.map_memory, my_position);

                // Optionally, plugins can be attached.
                let map = map
                    .with_plugin(super::plugins::places())
                    .with_plugin(super::plugins::images(&mut self.images_plugin_data))
                    .with_plugin(super::plugins::CustomShapes {})
                    .with_plugin(&mut self.click_watcher);

                // Draw the map widget.
                ui.add(map);

                // Draw utility windows.
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

                // Add incident menu
                egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
                    egui::menu::bar(ui, |ui| {
                        menu::bar(ui, |ui| {
                            ui.menu_button("Incidente", |ui| {
                                if !self.incident_dialog_open {
                                    if ui.button("Alta Incidente").clicked() {
                                        self.incident_dialog_open = true;
                                    }
                                }
                                if self.incident_dialog_open {
                                    ui.add_space(5.0);
                                    ui.horizontal(|ui| {
                                        ui.label("Latitud:");
                                        ui.text_edit_singleline(&mut self.latitude);
                                        ui.label("Longitud:");
                                        ui.text_edit_singleline(&mut self.longitude);
                                        if ui.button("OK").clicked() {
                                            let latitude_text = self.latitude.to_string();
                                            let longitude_text = self.longitude.to_string();

                                            println!("Latitud: {}", latitude_text);
                                            println!("Longitud: {}", longitude_text);

                                            let latitude = latitude_text.parse::<f32>().unwrap();
                                            let longitude: f32 =
                                                longitude_text.parse::<f32>().unwrap();
                                            let incident = Incident::new(0, latitude, longitude);
                                            // enviar el incidente a un channel
                                            self.send_incident(incident);
                                            self.incident_dialog_open = false;
                                        }
                                    });
                                }
                            });
                            if ui.button("Salir").clicked() {
                                // Handle exit
                            }
                        });
                    });
                });
            });
    }

    
}


