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
}

