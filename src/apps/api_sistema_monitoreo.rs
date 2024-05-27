use std::sync::{Arc, Mutex};

use super::incident::Incident;

#[derive(Debug, Clone)]
pub struct SistemaMonitoreo {
    pub incidents: Arc<Mutex<Vec<Incident>>>,
}

impl SistemaMonitoreo {
    pub fn new() -> Self {
        Self {
            incidents: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_incident(&self, incident: Incident) {
        self.incidents.lock().unwrap().push(incident);
    }

    pub fn get_incidents(&self) -> Vec<Incident> {
        self.incidents.lock().unwrap().clone()
    }

    pub fn remove_incident(&self, incident_id: u8) {
        let mut incidents = self.incidents.lock().unwrap();
        let index = incidents
            .iter()
            .position(|incident| incident.id == incident_id);
        if let Some(index) = index {
            incidents.remove(index);
        }
    }
}

impl Default for SistemaMonitoreo {
    fn default() -> Self {
        Self::new()
    }
}
