use super::incident_state::IncidentState;

#[derive(Debug, Clone)]
/// Struct que representa un incidente, para ser utilizado por las aplicaciones del sistema de vigilancia (sist de monitoreo, sist central de cámaras, y app de drones).
/// Posee un id, coordenadas x e y, un estado, y un campo `sent` que indica si el incidente se envió y continúa sin modificaciones desde entonces o si por el contrario ya se modificó desde la última vez que se envió.
pub struct Incident {
    pub id: u8, // []
    latitude: f32,
    longitude: f32,
    state: IncidentState,
    pub sent: bool,
}

impl Incident {
    pub fn new(id: u8, latitude: f32, longitude: f32) -> Self {
        Self {
            id,
            latitude,
            longitude,
            state: IncidentState::ActiveIncident,
            sent: false,
        }
    }

    /// Devuelve coordenadas (x, y) correspondientes a la posición del incidente.
    pub fn pos(&self) -> (f32, f32) {
        (self.latitude, self.longitude)
    }

    /// Devuelve si el incidente tiene estado resuelto o no.
    pub fn is_resolved(&self) -> bool {
        self.state == IncidentState::ResolvedIncident
    }

    /// Cambia el estado del incidente a resuelto.
    pub fn set_resolved(&mut self) {
        self.state = IncidentState::ResolvedIncident;
        self.sent = false;
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![self.id];
        bytes.extend_from_slice(&self.latitude.to_le_bytes());
        bytes.extend_from_slice(&self.longitude.to_le_bytes());
        bytes.push(self.state.to_byte()[0]);
        bytes
    }

    pub fn from_bytes(msg_bytes: Vec<u8>) -> Self {
        let id = msg_bytes[0];
        let latitude = f32::from_le_bytes([msg_bytes[1], msg_bytes[2], msg_bytes[3], msg_bytes[4]]);
        let longitude =
            f32::from_le_bytes([msg_bytes[5], msg_bytes[6], msg_bytes[7], msg_bytes[8]]);
        let mut state = IncidentState::ActiveIncident;
        if let Ok(state_parsed) = IncidentState::from_byte([msg_bytes[9]]) {
            state = state_parsed;
        }

        Self {
            id,
            latitude,
            longitude,
            state,
            sent: false,
        }
    }
}
// hacer test de los metodos from_bytes y to_bytes

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bytes() {
        let incident = Incident::from_bytes(vec![1, 0, 0, 0, 64, 0, 0, 0, 64, 1]);
        assert_eq!(incident.id, 1);
        assert_eq!(incident.latitude, 2.0);
        assert_eq!(incident.longitude, 2.0);
        assert_eq!(incident.state, IncidentState::ActiveIncident);
    }

    #[test]
    fn test_to_bytes() {
        let incident = Incident {
            id: 1,
            latitude: 2.0,
            longitude: 2.0,
            state: IncidentState::ActiveIncident,
            sent: false,
        };
        let bytes = incident.to_bytes();

        assert_eq!(bytes, vec![1, 0, 0, 0, 64, 0, 0, 0, 64, 1]);
    }

    #[test]
    fn test_reverse_from_bytes() {
        let incident = Incident::from_bytes(vec![1, 0, 0, 0, 64, 0, 0, 0, 64, 1]);
        let bytes = incident.to_bytes();
        assert_eq!(bytes, vec![1, 0, 0, 0, 64, 0, 0, 0, 64, 1]);
    }
    #[test]
    fn test_reverse_to_bytes() {
        let incident = Incident {
            id: 1,
            latitude: 2.0,
            longitude: 2.0,
            state: IncidentState::ActiveIncident,
            sent: false,
        };
        let bytes = incident.to_bytes();
        let incident_bytes = Incident::from_bytes(bytes);
        assert_eq!(incident_bytes.id, incident.id);
        assert_eq!(incident_bytes.latitude, incident.latitude);
        assert_eq!(incident_bytes.longitude, incident.longitude);
        assert_eq!(incident_bytes.state, incident.state);
    }
}