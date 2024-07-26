use crate::mqtt::mqtt_utils::will_message_utils::app_type::AppType;

use super::incident_data::incident_source::IncidentSource;

/// PlaceType para usar en el vector que utiliza la ui de Sistema monitoreo.
/// Según este enum se identifican los elementos en dicho vector, ya que el mismo puede tener
/// elementos con un mismo id pero diferente place type (por ejemplo cámara 1 y dron 1).
#[derive(Debug, PartialEq, Clone)]
pub enum PlaceType {
    Camera,
    Dron,
    ManualIncident,
    AutomatedIncident,
    Mantainance,
}

impl PlaceType {
    /// Devuelve un `PlaceType` acorde al `IncidentSource` recibido.
    pub fn from_inc_source(source: &IncidentSource) -> Self {
        match source {
            IncidentSource::Manual => Self::ManualIncident,
            IncidentSource::Automated => Self::AutomatedIncident,
        }
    }

    /// Devuelve un `PlaceType` acorde al `AppType` del `WillContent` recibido.
    pub fn from_app_type_will_content(app_type: &AppType) -> Self {
        match app_type {
            AppType::Cameras => Self::Camera,
            AppType::Dron => Self::Dron,
            AppType::Monitoreo => Self::Mantainance, // Aux: esta rama no tiene sentido, nunca se va a dar. Ver. [].
        }
    }
}
