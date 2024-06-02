use egui::{pos2, Color32, Context, Mesh, Rect, Vec2};
use egui::{ColorImage, TextureHandle};
use image::ImageError;

use super::download::{download_continuously, HttpOptions};
use super::io::Runtime;
use super::limited_map::LimitedMap;
use super::mercator::TileId;
use super::sources::{Attribution, TileSource};

pub fn rect(screen_position: Vec2, tile_size: f64) -> Rect {
    Rect::from_min_size(screen_position.to_pos2(), Vec2::splat(tile_size as f32))
}

#[derive(Clone)]
pub struct Texture(TextureHandle);

impl Texture {
    pub fn new(image: &[u8], ctx: &Context) -> Result<Self, ImageError> {
        let image = image::load_from_memory(image)?.to_rgba8();
        let pixels = image.as_flat_samples();
        let image = ColorImage::from_rgba_unmultiplied(
            [image.width() as _, image.height() as _],
            pixels.as_slice(),
        );

        Ok(Self::from_color_image(image, ctx))
    }

    /// Load the texture from egui's [`ColorImage`].
    pub fn from_color_image(color_image: ColorImage, ctx: &Context) -> Self {
        Self(ctx.load_texture("image", color_image, Default::default()))
    }

    pub fn size(&self) -> Vec2 {
        self.0.size_vec2()
    }

    pub fn mesh(&self, screen_position: Vec2, tile_size: f64) -> Mesh {
        self.mesh_with_rect(rect(screen_position, tile_size))
    }

    pub fn mesh_with_rect(&self, rect: Rect) -> Mesh {
        let mut mesh = Mesh::with_texture(self.0.id());
        mesh.add_rect_with_uv(
            rect,
            Rect::from_min_max(pos2(0., 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        mesh
    }
}

pub trait TilesManager {
    fn at(&mut self, tile_id: TileId) -> Option<Texture>;
    fn attribution(&self) -> Attribution;
    fn tile_size(&self) -> u32;
}

/// Downloads the tiles via HTTP. It must persist between frames.
pub struct Tiles {
    attribution: Attribution,

    cache: LimitedMap<TileId, Option<Texture>>,

    /// Tiles to be downloaded by the IO thread.
    request_tx: futures::channel::mpsc::Sender<TileId>,

    /// Tiles that got downloaded and should be put in the cache.
    tile_rx: futures::channel::mpsc::Receiver<(TileId, Texture)>,

    #[allow(dead_code)] // Significant Drop
    runtime: Runtime,

    tile_size: u32,
}

impl Tiles {
    /// Construct new [`Tiles`] with default [`HttpOptions`].
    pub fn new<S>(source: S, egui_ctx: Context) -> Self
    where
        S: TileSource + Send + 'static,
    {
        Self::with_options(source, HttpOptions::default(), egui_ctx)
    }

    /// Construct new [`Tiles`] with supplied [`HttpOptions`].
    pub fn with_options<S>(source: S, http_options: HttpOptions, egui_ctx: Context) -> Self
    where
        S: TileSource + Send + 'static,
    {
        // Minimum value which didn't cause any stalls while testing.
        let channel_size = 20;

        let (request_tx, request_rx) = futures::channel::mpsc::channel(channel_size);
        let (tile_tx, tile_rx) = futures::channel::mpsc::channel(channel_size);
        let attribution = source.attribution();
        let tile_size = source.tile_size();

        let runtime = Runtime::new(download_continuously(
            source,
            http_options,
            request_rx,
            tile_tx,
            egui_ctx,
        ));

        Self {
            attribution,
            cache: LimitedMap::new(256), // Just arbitrary value which seemed right.
            request_tx,
            tile_rx,
            runtime,
            tile_size,
        }
    }
}

impl TilesManager for Tiles {
    /// Attribution of the source this tile cache pulls images from. Typically,
    /// this should be displayed somewhere on the top of the map widget.
    fn attribution(&self) -> Attribution {
        self.attribution.clone()
    }

    /// Return a tile if already in cache, schedule a download otherwise.
    fn at(&mut self, tile_id: TileId) -> Option<Texture> {
        // Just take one at the time.
        match self.tile_rx.try_next() {
            Ok(Some((tile_id, tile))) => {
                self.cache.insert(tile_id, Some(tile));
            }
            Err(_) => {
                // Just ignore. It means that no new tile was downloaded.
            }
            Ok(None) => {
                log::error!("IO thread is dead")
            }
        }

        // TODO: Double lookup.
        if let Some(texture) = self.cache.get(&tile_id).cloned() {
            texture
        } else {
            if let Ok(()) = self.request_tx.try_send(tile_id) {
                log::debug!("Requested tile: {:?}", tile_id);
                self.cache.insert(tile_id, None);
            } else {
                log::debug!("Request queue is full.");
            }
            None
        }
    }

    fn tile_size(&self) -> u32 {
        self.tile_size
    }
}
