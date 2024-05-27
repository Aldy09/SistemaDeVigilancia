use std::sync::{Arc, Mutex};

use super::incident::Incident;

#[derive(Debug)]
pub struct SistemaMonitoreo {
    pub incidents: Arc<Mutex<Vec<Incident>>>,
}

impl SistemaMonitoreo {
    pub fn new() -> Self {
        Self {
            incidents: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_incident(&mut self, incident: Incident) {
        self.incidents.lock().unwrap().push(incident);
    }

    pub fn get_incidents(&mut self) -> Arc<Mutex<Vec<Incident>>> {
        self.incidents.clone()
    }

    pub fn generate_new_incident_id(&self) -> u8 {
        let mut new_inc_id: u8 = 0;
        if let Ok(incidents) = self.incidents.lock() {
            new_inc_id = (incidents.len() + 1) as u8;
        }
        new_inc_id
    }
}

impl Default for SistemaMonitoreo {
    fn default() -> Self {
        Self::new()
    }
}
