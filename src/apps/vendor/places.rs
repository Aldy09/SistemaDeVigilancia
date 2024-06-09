use egui::{vec2, Align2, Color32, FontId, Painter, Response, Stroke};

use super::{Plugin, Position};

/// Visual style of the place.
#[derive(Debug, Clone)]
pub struct Style {
    pub label_font: FontId,
    pub label_color: Color32,
    pub label_background: Color32,
    pub symbol_font: FontId,
    pub symbol_color: Color32,
    pub symbol_background: Color32,
    pub symbol_stroke: Stroke,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            label_font: FontId::proportional(12.),
            label_color: Color32::from_gray(200),
            label_background: Color32::BLACK.gamma_multiply(0.8),
            symbol_font: FontId::proportional(30.),
            symbol_color: Color32::BLACK.gamma_multiply(0.8),
            symbol_background: Color32::WHITE.gamma_multiply(0.8),
            symbol_stroke: Stroke::new(2., Color32::BLACK.gamma_multiply(0.8)),
        }
    }
}

#[derive(Debug, Clone)]
/// A place to be drawn on the map.
pub struct Place {
    /// Geographical position.
    pub position: Position,

    /// Text displayed next to the marker.
    pub label: String,

    /// Symbol drawn on the place. You can check [egui's font book](https://www.egui.rs/) to pick
    /// a proper character.
    pub symbol: char,

    /// Visual style of this place.
    pub style: Style,

    /// Unique identifier of the place.
    pub id: u8,
}

impl Place {
    fn draw(&self, _response: &Response, painter: Painter, projector: &super::Projector) {
        let screen_position = projector.project(self.position);

        let label = painter.layout_no_wrap(
            self.label.to_owned(),
            self.style.label_font.clone(),
            self.style.label_color,
        );

        // Offset of the label, relative to the circle.
        let offset = vec2(8., 8.);

        painter.rect_filled(
            label
                .rect
                .translate(screen_position)
                .translate(offset)
                .expand(5.),
            10.,
            self.style.label_background,
        );

        painter.galley(
            (screen_position + offset).to_pos2(),
            label,
            egui::Color32::BLACK,
        );

        painter.circle(
            screen_position.to_pos2(),
            25.,
            self.style.symbol_background,
            self.style.symbol_stroke,
        );

        painter.text(
            screen_position.to_pos2(),
            Align2::CENTER_CENTER,
            self.symbol.to_string(),
            self.style.symbol_font.clone(),
            self.style.symbol_color,
        );
    }
}

/// [`Plugin`] which draws list of places on the map.
///
#[derive(Debug, Clone)]
pub struct Places {
    places: Vec<Place>,
}

impl Places {
    pub fn new() -> Self {
        Self { places: Vec::new() }
    }

    pub fn add_place(&mut self, place: Place) {
        self.places.push(place);
    }

    pub fn get_places(&self) -> Vec<Place> {
        self.places.to_vec()
    }

    pub fn remove_place(&mut self, id: u8) {
        if let Some(index) = self.places.iter().position(|p| p.id == id) {
            self.places.remove(index);
        } else {
            println!("No se encontró un lugar con el id {}", id);
        }
    }
}

impl Plugin for Places {
    fn run(&mut self, response: &Response, painter: Painter, projector: &super::Projector) {
        for place in &self.places {
            place.draw(response, painter.clone(), projector);
        }
    }
}

impl Default for Places {
    fn default() -> Self {
        Self::new()
    }
}
