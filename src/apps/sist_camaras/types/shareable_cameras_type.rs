use std::{collections::HashMap, sync::{Arc, Mutex}};

use super::super::camera::Camera;

pub type ShCamerasType = Arc<Mutex<HashMap<u8, Camera>>>;