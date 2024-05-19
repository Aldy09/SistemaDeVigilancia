use super::incident_state::IncidentState;

#[derive(Debug)]
/// Struct que representa un incidente, para ser utilizado por las aplicaciones del sistema de vigilancia (sist de monitoreo, sist central de cámaras, y app de drones).
/// Posee un id, coordenadas x e y, un estado, y un campo `sent` que indica si el incidente se envió y continúa sin modificaciones desde entonces o si por el contrario ya se modificó desde la última vez que se envió.
pub struct Incident {
    pub id: u8, // []
    coord_x: u8,
    coord_y: u8,
    state: IncidentState,
    sent: bool,
}

impl Incident {
    pub fn new(id: u8, coord_x: u8, coord_y: u8) -> Self {
        Self { id, coord_x, coord_y, state: IncidentState::ActiveIncident, sent: false}
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


}