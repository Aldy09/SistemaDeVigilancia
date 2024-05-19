use super::incident_state::IncidentState;
#[allow(dead_code)]

#[derive(Debug)]
pub struct Incident {
    pub id: u8, // []
    coord_x: u8,
    coord_y: u8,
    state: IncidentState,
}

impl Incident {
    pub fn new(id: u8, coord_x: u8, coord_y: u8) -> Self {
        Self { id, coord_x, coord_y, state: IncidentState::ActiveIncident }
    }

    /// Devuelve coordenadas (x, y) correspondientes a la posición del incidente.
    pub fn pos(&self) -> (u8, u8) {
        (self.coord_x, self.coord_y)
    }


}