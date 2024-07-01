use std::sync::mpsc::Sender;


#[derive(Debug)]
pub struct StringLogger {
    tx: Sender<String>,
}

impl StringLogger {

    
    pub fn new(tx: Sender<String>) -> Self {
        StringLogger {tx}
    }

    pub fn log(&self, event: String) {
        if self.tx.send(event).is_err() {
            // Aux: el tx podría estar en un logger, y llamar logger.log(string) x ej.
            println!("Cliente: Error al intentar loggear.");
        }
    }
    
    pub fn clone_ref(&self) -> StringLogger {
        Self::new(self.tx.clone())
    }
    
}