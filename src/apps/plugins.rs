use super::vendor::{Image, Images, Texture};
use super::vendor::{Plugin, Position, Projector};
use egui::{Color32, Painter, Response};

use super::places;

/// Creates a built-in `Places` plugin with some predefined places.
// pub fn places() -> impl Plugin {
//     Places::new(vec![
// Place {
//     position: places::obelisco(),
//     label: "Wrocław Główny\ntrain station".to_owned(),
//     symbol: '📷',
//     style: Style::default(),
// },
//         Place {
//             position: places::dworcowa_bus_stop(),
//             label: "Bus stop".to_owned(),
//             symbol: '🚌',
//             style: Style::default(),
//         },
//     ])
// }

/// Estructura de datos para el plugin de imagenes.
pub struct ImagesPluginData {
    pub texture: Texture,
    pub angle: f32,
    pub x_scale: f32,
    pub y_scale: f32,
}

impl ImagesPluginData {
    pub fn new(egui_ctx: egui::Context) -> Self {
        Self {
            texture: Texture::from_color_image(egui::ColorImage::example(), &egui_ctx),
            angle: 0.0,
            x_scale: 1.0,
            y_scale: 1.0,
        }
    }
}

/// Creates a built-in `Images` plugin with an example image.
pub fn images(images_plugin_data: &mut ImagesPluginData) -> impl Plugin {
    Images::new(vec![{
        let mut image = Image::new(images_plugin_data.texture.clone(), places::wroclavia());
        image.scale(images_plugin_data.x_scale, images_plugin_data.y_scale);
        image.angle(images_plugin_data.angle.to_radians());
        image
    }])
}

/// Estructura de datos para el plugin de formas personalizadas.
pub struct CustomShapes {}

impl Plugin for CustomShapes {
    /// Dibuja un circulo en la posicion del obelisco.
    fn run(&mut self, response: &Response, painter: Painter, projector: &Projector) {
        // Posicion del obelisco.
        let position = places::capitol();

        // Project it into the position on the screen.
        let position = projector.project(position).to_pos2();

        let radius = 30.;

        let hovered = response
            .hover_pos()
            .map(|hover_pos| hover_pos.distance(position) < radius)
            .unwrap_or(false);

        painter.circle_filled(
            position,
            radius,
            Color32::BLACK.gamma_multiply(if hovered { 0.5 } else { 0.2 }),
        );
    }
}

#[derive(Default, Clone)]
pub struct ClickWatcher {
    pub clicked_at: Option<Position>,
}

impl ClickWatcher {
    pub fn show_position(&self, ui: &egui::Ui) {
        if let Some(clicked_at) = self.clicked_at {
            egui::Window::new("Clicked Position")
                .collapsible(false)
                .resizable(false)
                .title_bar(false)
                .anchor(egui::Align2::CENTER_BOTTOM, [0., -10.])
                .show(ui.ctx(), |ui| {
                    // Muestro la posicion seleccionada como latitud y longitud.
                    ui.label(format!("{:.04} {:.04}", clicked_at.lat(), clicked_at.lon()))
                        .on_hover_text("last clicked position");
                });
        }
    }
}

impl Plugin for &mut ClickWatcher {
    fn run(&mut self, response: &Response, painter: Painter, projector: &Projector) {
        if !response.changed() && response.clicked_by(egui::PointerButton::Primary) {
            self.clicked_at = response
                .interact_pointer_pos()
                .map(|p| projector.unproject(p - response.rect.center()));
        }

        if let Some(position) = self.clicked_at {
            painter.circle_filled(projector.project(position).to_pos2(), 5.0, Color32::BLUE);
        }
    }
}
