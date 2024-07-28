use config::{Config, File};

#[derive(Debug)]
#[allow(dead_code)]
pub struct ApiCredentials {
    prediction_key: String,
    endpoint: String,
}

impl ApiCredentials {
    pub fn new() -> Self {
        // Inicializar la configuración
        let aux = "./src/apps/sist_camaras/azure_model/key_and_endpoint";
        let mut settings = Config::default();
        settings
            .merge(File::with_name(aux))
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

impl Default for ApiCredentials {
    fn default() -> Self {
        Self::new()
    }
}
