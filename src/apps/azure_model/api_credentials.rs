#[derive(Debug)]
#[allow(dead_code)]
pub struct ApiCredentials {
    prediction_key: String,
    endpoint: String,
}

impl ApiCredentials {
    pub fn new() -> Self {
        // Inicializar la configuración
        let mut settings = Config::default();
        settings
            .merge(File::with_name("key_and_endpoint"))
            .expect("Error al cargar el archivo de configuración");
    
        // Leer las propiedades
        let prediction_key: String = settings
            .get("prediction_key")
            .expect("Error al leer prediction_key");
        let endpoint: String = settings.get("endpoint").expect("Error al leer endpoint");

        Self {
            prediction_key,
            endpoint,
        }
    }

    pub fn get_prediction_key(&self) -> String {
        self.prediction_key
    }

    pub fn get_endpoint(&self) -> String {
        self.endpoint
    }
    
}