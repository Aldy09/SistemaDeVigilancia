use std::sync::mpsc::Sender;


#[derive(Debug)]
pub struct StringLogger {
    tx: Sender<String>,
}

impl StringLogger {

    /// Extremo de envío del string logger.
    /// Es el encargado de enviar las strings a ser loggueadas.
    pub fn new(tx: Sender<String>) -> Self {
        StringLogger {tx}
    }

    // Ejemplo: logger.log(format!("Ha ocurrido un evento: {}", string_event));
    /// Función a llamar para grabar en el log el evento pasado por parámetro.
    pub fn log(&self, event: String) {
        if self.tx.send(event).is_err() {
            println!("Cliente: Error al intentar loggear.");
        }
    }
    
    pub fn clone_ref(&self) -> StringLogger {
        Self::new(self.tx.clone())
    }
    
}