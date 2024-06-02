pub struct InvalidZoom;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Zoom(f32);

impl TryFrom<f32> for Zoom {
    type Error = InvalidZoom;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        // Mapnik supports zooms up to 19.
        // https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames#Zoom_levels
        if !(0. ..=19.).contains(&value) {
            Err(InvalidZoom)
        } else {
            Ok(Self(value))
        }
    }
}

// The reverse shouldn't be implemented, since we already have TryInto<f32>.
#[allow(clippy::from_over_into)]
impl Into<f64> for Zoom {
    fn into(self) -> f64 {
        self.0 as f64
    }
}

impl Default for Zoom {
    fn default() -> Self {
        Self(16.)
    }
}

impl Zoom {
    pub fn round(&self) -> u8 {
        self.0.round() as u8
    }

    pub fn zoom_in(&mut self) -> Result<(), InvalidZoom> {
        *self = Self::try_from(self.0 + 1.)?;
        Ok(())
    }

    pub fn zoom_out(&mut self) -> Result<(), InvalidZoom> {
        *self = Self::try_from(self.0 - 1.)?;
        Ok(())
    }

    /// Zoom using a relative value.
    pub fn zoom_by(&mut self, value: f32) {
        if let Ok(new_self) = Self::try_from(self.0 + value) {
            *self = new_self;
        }
    }
}

