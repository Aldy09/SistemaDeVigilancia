use std::sync::{Mutex, Arc};

use super::incident::Incident;

#[derive(Debug, Clone)]
pub struct SistemaMonitoreo {
    //pub incidents: Vec<Incident>,
    pub incidents: Arc<Mutex<Vec<Incident>>>,
}

impl SistemaMonitoreo {
    pub fn new() -> Self {
        Self {
            //incidents: Vec::new(),
            incidents: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_incident(&mut self, incident: Incident) {
        self.incidents.lock().unwrap().push(incident);
        //self.incidents.push(incident);
    }

    pub fn get_incidents(&mut self) -> Arc<Mutex<Vec<Incident>>>  {//-> &Vec<Incident> {
        self.incidents.clone()
        //self.incidents.lock().unwrap().clone()
        //&self.incidents
    }

    /// Marca un inidente como enviado.
    pub fn mark_incident_as_sent(&mut self, inc_id: u8){
        /*for incident in &mut self.incidents {
            if incident.id == inc_id {
                incident.sent = true;
            }
        }*/

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
