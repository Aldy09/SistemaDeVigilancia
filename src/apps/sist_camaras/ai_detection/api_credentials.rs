use config::{Config, File};

#[derive(Debug)]
pub struct ApiCredentials {
    prediction_key: String,
    endpoint: String,
}

impl ApiCredentials {
    pub fn new(key_endpoint_path: String) -> Self {
        // Inicializar la configuración
        let mut settings = Config::default();
        settings
            .merge(File::with_name(key_endpoint_path.as_str()))
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
        self.prediction_key.clone()
    }

    pub fn get_endpoint(&self) -> String {
        self.endpoint.clone()
    }
}

/*impl Default for ApiCredentials {
    fn default() -> Self {
        Self::new()
    }
}*/
