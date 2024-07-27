use std::io::Error;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AppType {
    Cameras,
    Dron,
    Monitoreo
}

impl AppType {
    pub fn to_str(&self) -> String {
        match self {
            AppType::Cameras => String::from("camaras"),
            AppType::Dron => String::from("dron"),
            AppType::Monitoreo => String::from("monitoreo"),
        }
    }

    pub fn app_type_from_str(str: &str) -> Result<Self, Error> {
        match str {
            "camaras" => Ok(AppType::Cameras),
            "dron" => Ok(AppType::Dron),
            "monitoreo" => Ok(AppType::Monitoreo),
            _ => Err(Error::new(std::io::ErrorKind::InvalidInput, "Error: string inválida para crea un enum AppType."))

        }
    }
}