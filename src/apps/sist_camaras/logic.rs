use std::collections::HashMap;

use super::{hashmap_incs_type::HashmapIncsType, shareable_cameras_type::ShCamerasType};

#[derive(Debug)]
pub struct CamerasLogic {
    cameras: ShCamerasType,
    incs_being_managed: HashmapIncsType,
}

impl CamerasLogic {
    /// Crea un struct CamerasLogic con las cámaras pasadas como parámetro e incidentes manejándose vacíos.
    pub fn new(cameras: ShCamerasType) -> Self {
        Self { cameras, incs_being_managed: HashMap::new() }
    }
}