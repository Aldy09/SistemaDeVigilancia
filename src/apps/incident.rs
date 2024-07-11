use std::io::Error;

use super::incident_info::IncidentInfo;
use super::incident_state::IncidentState;
use super::incident_source::IncidentSource;

#[derive(Debug, Clone)]
/// Struct que representa un incidente, para ser utilizado por las aplicaciones del sistema de vigilancia (sist de monitoreo, sist central de cámaras, y app de drones).
/// Posee un id, coordenadas x e y, un estado.
pub struct Incident {
    id: u8, // []
    latitude: f64,
    longitude: f64,
    state: IncidentState,
    source: IncidentSource,
}

impl Incident {
    pub fn new(id: u8, latitude: f64, longitude: f64, source: IncidentSource) -> Self {
        Self {
            id,
            latitude,
            longitude,
            state: IncidentState::ActiveIncident,
            source,
        }
    }

    /// Devuelve coordenadas (x, y) correspondientes a la posición del incidente.
    pub fn get_position(&self) -> (f64, f64) {
        (self.latitude, self.longitude)
    }

    /// Devuelve si el incidente tiene estado resuelto o no.
    pub fn is_resolved(&self) -> bool {
        self.state == IncidentState::ResolvedIncident
    }

    /// Cambia el estado del incidente a resuelto.
    pub fn set_resolved(&mut self) {
        self.state = IncidentState::ResolvedIncident;
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![self.id];
        bytes.extend_from_slice(&self.latitude.to_le_bytes());
        bytes.extend_from_slice(&self.longitude.to_le_bytes());
        bytes.push(self.state.to_byte()[0]);
        bytes.push(self.source.to_byte()[0]);
        bytes
    }

    pub fn get_id(&self) -> u8 {
        self.id
    }

    pub fn get_info(&self) -> IncidentInfo {
        IncidentInfo::new(self.id, self.source)
    }

    pub fn from_bytes(msg_bytes: Vec<u8>) -> Result<Self, Error> {
        let id = msg_bytes[0];
        let latitude = f64::from_le_bytes([
            msg_bytes[1],
            msg_bytes[2],
            msg_bytes[3],
            msg_bytes[4],
            msg_bytes[5],
            msg_bytes[6],
            msg_bytes[7],
            msg_bytes[8],
        ]);
        let longitude = f64::from_le_bytes([
            msg_bytes[9],
            msg_bytes[10],
            msg_bytes[11],
            msg_bytes[12],
            msg_bytes[13],
            msg_bytes[14],
            msg_bytes[15],
            msg_bytes[16],
        ]);
        
        let state = IncidentState::from_byte([msg_bytes[17]])?;

        let source = IncidentSource::from_byte([msg_bytes[18]])?;

        Ok(Self {
            id,
            latitude,
            longitude,
            state,
            source,
        })
    }

    /// Devuelve el estado del incidente.
    pub fn get_state(&self) -> &IncidentState {
        &self.state
    }
    
    pub fn get_source(&self) -> &IncidentSource {
        &self.source
    }
}
// hacer test de los metodos from_bytes y to_bytes

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverse_to_bytes() {
        let incident = Incident {
            id: 1,
            latitude: 2.0,
            longitude: 2.0,
            state: IncidentState::ActiveIncident,
            source: IncidentSource::Manual,
        };
        let bytes = incident.to_bytes();
        let incident_bytes = Incident::from_bytes(bytes).unwrap();
        assert_eq!(incident_bytes.id, incident.id);
        assert_eq!(incident_bytes.latitude, incident.latitude);
        assert_eq!(incident_bytes.longitude, incident.longitude);
        assert_eq!(incident_bytes.state, incident.state);
    }
}
