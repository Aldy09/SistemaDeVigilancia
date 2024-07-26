use std::io::Error;

#[derive(Debug)]
pub enum AppsMqttTopics {
    IncidentTopic,
    DronTopic,
    CameraTopic,
    DescTopic,
}

impl AppsMqttTopics {
    pub fn to_str(&self) -> &str {
        match self {
            AppsMqttTopics::IncidentTopic => "inc",
            AppsMqttTopics::DronTopic => "dron",
            AppsMqttTopics::CameraTopic => "cam",
            AppsMqttTopics::DescTopic => "desc",
        }
    }

    pub fn topic_from_str(str: &str) -> Result<Self, Error> {
        match str {
            "inc" => Ok(AppsMqttTopics::IncidentTopic),
            "dron" => Ok(AppsMqttTopics::DronTopic),
            "cam" => Ok(AppsMqttTopics::CameraTopic),
            "desc" => Ok(AppsMqttTopics::DescTopic),
            _ => Err(Error::new(std::io::ErrorKind::InvalidInput, "Error: string inválida para crea un enum AppsMqttTopics."))

        }
    }
}
