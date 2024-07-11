use crate::apps::incident_data::incident::Incident;
use crate::apps::sist_camaras::camera::Camera;

#[derive(Debug)]
pub enum AppType {
    Camera(Camera),
    Incident(Incident),
}
