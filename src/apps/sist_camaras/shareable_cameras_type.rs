use std::{collections::HashMap, sync::{Arc, Mutex}};

use super::camera::Camera;

type ShareableCamType = Camera;
pub type ShCamerasType = Arc<Mutex<HashMap<u8, ShareableCamType>>>;