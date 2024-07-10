use super::incident_source::IncidentSource;

/// Este struct se utiliza como clave en hashmaps para identificar a un Incident.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct IncidentInfo {
    inc_id: u8,
    src: IncidentSource,
}
impl IncidentInfo {
    pub fn new(inc_id: u8, src: IncidentSource) -> Self {
        Self {inc_id, src}
    }
    pub fn get_inc_id(&self) -> u8 {
        self.inc_id
    }
    pub fn get_src(&self) -> &IncidentSource {
        &self.src
    }
}