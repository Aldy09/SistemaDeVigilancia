use crate::apps::sist_camaras::camera::Camera;
use crate::apps::incident::Incident;

#[derive(Debug)]
pub enum AppType {
    Camera(Camera),
    Incident(Incident),
}
