use egui::ColorImage;
use egui::Context;
use super::vendor::sources::Attribution;
use super::vendor::Texture;
use super::vendor::TileId;
use super::vendor::TilesManager;

pub struct LocalTiles {
    egui_ctx: Context,
}

impl LocalTiles {
    pub fn new(egui_ctx: Context) -> Self {
        Self { egui_ctx }
    }
}

impl TilesManager for LocalTiles {
    fn at(&mut self, _tile_id: TileId) -> Option<Texture> {
        let image = ColorImage::example();

        Some(Texture::from_color_image(image, &self.egui_ctx))
    }

    fn attribution(&self) -> Attribution {
        Attribution {
            text: "Local rendering example",
            url: "https://github.com/podusowski/walkers",
            logo_light: None,
            logo_dark: None,
        }
    }

    fn tile_size(&self) -> u32 {
        256
    }
}
