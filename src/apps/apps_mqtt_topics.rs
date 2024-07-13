use std::io::Error;

#[derive(Debug)]
pub enum AppsMqttTopics {
    IncidentTopic,
    DronTopic,
    CameraTopic,
}

impl AppsMqttTopics {
    pub fn to_str(&self) -> &str {
        match self {
            AppsMqttTopics::IncidentTopic => "Inc",
            AppsMqttTopics::DronTopic => "Dron",
            AppsMqttTopics::CameraTopic => "Cam",
        }
    }

    pub fn topic_from_str(str: &str) -> Result<Self, Error> {
        match str {
            "Inc" => Ok(AppsMqttTopics::IncidentTopic),
            "Dron" => Ok(AppsMqttTopics::DronTopic),
            "Cam" => Ok(AppsMqttTopics::CameraTopic),
            _ => Err(Error::new(std::io::ErrorKind::InvalidInput, "Error: string inválida para crea un enum AppsMqttTopics."))

        }
    }
}
