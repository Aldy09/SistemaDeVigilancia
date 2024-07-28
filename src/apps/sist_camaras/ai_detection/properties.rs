use std::io::{Error, ErrorKind};

use crate::apps::properties::Properties;

#[derive(Debug, PartialEq, Clone)]
pub struct DetectorProperties {
    base_dir: String,
    api_credentials_file_path: String,
    inc_tag: String,
    inc_threshold: f64,
}

impl DetectorProperties {
    pub fn new(properties_file: &str) -> Result<Self, Error> {
        // Cargamos todas las properties (constantes) del archivo, a este global_properties, por simplicidad de lectura
        let global_properties = Properties::new(properties_file)?;

        // Y ahora buscamos las properties, y las cargamos a los campos de este struct
        let base_dir: String;
        if let Some(prop) = global_properties.get("base_dir") {
            base_dir = String::from(prop);
        } else {
            println!("No se encontró la propiedad 'base_dir");
            return Err(Error::new(
                ErrorKind::Other,
                "Falta propiedad base_dir.",
            ));
        }

        let api_credentials_file_path: String;
        if let Some(prop) = global_properties.get("api_credentials_file_path") {
            api_credentials_file_path = String::from(prop);
        } else {
            println!("No se encontró la propiedad 'api_credentials_file_path");
            return Err(Error::new(
                ErrorKind::Other,
                "Falta propiedad api_credentials_file_path.",
            ));
        }

        let inc_tag: String;
        if let Some(prop) = global_properties.get("inc_tag") {
            inc_tag = String::from(prop);
        } else {
            println!("No se encontró la propiedad 'inc_tag");
            return Err(Error::new(
                ErrorKind::Other,
                "Falta propiedad inc_tag.",
            ));
        }

        let inc_threshold: f64;
        if let Some(prop) = global_properties.get("inc_threshold") {
            inc_threshold = prop
                .parse()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "inc_threshold"))?;
        } else {
            println!("No se encontró la propiedad 'inc_threshold");
            return Err(Error::new(ErrorKind::Other, "Falta propiedad inc_threshold."));
        }

        Ok(Self {
            base_dir,
            api_credentials_file_path,
            inc_tag,
            inc_threshold,
        })
    }

    /// Devuelve el directorio base que contendrá a todos los subdirectorios de las cámaras.
    pub fn get_base_dir(&self) -> &str {
        self.base_dir.as_str()
    }

    /// Devuelve el archivo de configuración de api credentials (key_and_endpoint).
    pub fn get_api_credentials_file_path(&self) -> String {
        self.api_credentials_file_path.clone()
    }
    
    /// Devuelve el tag a buscar en la response del proveedor de inteligencia artifial.
    /// que indica si la imagen contiene o no un incidente.
    pub fn get_inc_tag(&self) -> &String {
        &self.inc_tag
    }

    /// Devuelve el umbral a utilizar para analizar si la probabilidad de ser incidente
    /// es o no la que consideramos suficiente para declarar a la situación como incidente.
    pub fn get_inc_threshold(&self) -> f64 {
        self.inc_threshold
    }
}
