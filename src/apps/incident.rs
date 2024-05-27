use super::incident_state::IncidentState;

#[derive(Debug, Clone)]
/// Struct que representa un incidente, para ser utilizado por las aplicaciones del sistema de vigilancia (sist de monitoreo, sist central de cámaras, y app de drones).
/// Posee un id, coordenadas x e y, un estado, y un campo `sent` que indica si el incidente se envió y continúa sin modificaciones desde entonces o si por el contrario ya se modificó desde la última vez que se envió.
pub struct Incident {
    pub id: u8, // []
    coord_x: u8,
    coord_y: u8,
    state: IncidentState,
    pub sent: bool,
}

impl Incident {
    pub fn new(id: u8, coord_x: u8, coord_y: u8) -> Self {
        Self {
            id,
            coord_x,
            coord_y,
            state: IncidentState::ActiveIncident,
            sent: false,
        }
    }

    /// Devuelve coordenadas (x, y) correspondientes a la posición del incidente.
    pub fn pos(&self) -> (u8, u8) {
        (self.coord_x, self.coord_y)
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
        vec![self.id, self.coord_x, self.coord_y, self.state.to_byte()[0]]
    }

    pub fn from_bytes(msg_bytes: Vec<u8>) -> Self {
        let id = msg_bytes[0];
        let coord_x = msg_bytes[1];
        let coord_y = msg_bytes[2];
        let mut state = IncidentState::ActiveIncident;
        if let Ok(state_parsed) = IncidentState::from_byte([msg_bytes[3]]) {
            state = state_parsed;
        }

        Self {
            id,
            coord_x,
            coord_y,
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
        let incident = Incident::from_bytes(vec![1, 2, 3, 1]);
        assert_eq!(incident.id, 1);
        assert_eq!(incident.coord_x, 2);
        assert_eq!(incident.coord_y, 3);
        assert_eq!(incident.state, IncidentState::ActiveIncident);
    }

    #[test]
    fn test_to_bytes() {
        let incident = Incident {
            id: 1,
            coord_x: 2,
            coord_y: 3,
            state: IncidentState::ActiveIncident,
            sent: false,
        };
        let bytes = incident.to_bytes();

        assert_eq!(bytes, vec![1, 2, 3, 1]);
    }

    #[test]
    fn test_reverse_from_bytes() {
        let incident = Incident::from_bytes(vec![1, 2, 3, 1]);
        let bytes = incident.to_bytes();
        assert_eq!(bytes, vec![1, 2, 3, 1]);
    }
    #[test]
    fn test_reverse_to_bytes() {
        let incident = Incident {
            id: 1,
            coord_x: 2,
            coord_y: 3,
            state: IncidentState::ActiveIncident,
            sent: false,
        };
        let bytes = incident.to_bytes();
        let incident_bytes = Incident::from_bytes(bytes);
        assert_eq!(incident_bytes.id, incident.id);
        assert_eq!(incident_bytes.coord_x, incident.coord_x);
        assert_eq!(incident_bytes.coord_y, incident.coord_y);
        assert_eq!(incident_bytes.state, incident.state);
    }
}
