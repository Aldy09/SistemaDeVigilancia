use std::sync::{Arc, Mutex};

use crate::{logging::string_logger::StringLogger, mqtt::client::mqtt_client::MQTTClient};

use super::{data::Data, dron::Dron, sist_dron_properties::SistDronProperties};

#[derive(Debug)]
pub struct DronLogic {
    current_data: Data,
    dron_properties: SistDronProperties,
    mqtt_client: Arc<Mutex<MQTTClient>>,
    logger: StringLogger,
    dron: Dron, // Aux: el dron y el mqtt_client solo se guardan acá mientras dura el refactor, dsp veremos.
                // aux: ahora se está usando solamente para llamar a publish_current_info.
}

impl DronLogic {
    pub fn new(current_data: Data, dron_properties: SistDronProperties, mqtt_client: Arc<Mutex<MQTTClient>>, logger: StringLogger, dron: Dron) -> Self {
        Self { current_data, dron_properties, mqtt_client, logger, dron }
    }

    pub fn run() {

    }

    pub fn clone_ref(&self) -> Self {
        Self { current_data: self.current_data.clone_ref(),
            dron_properties: self.dron_properties,
            mqtt_client: self.mqtt_client.clone(),
            logger: self.logger.clone_ref(),
            dron: self.dron.clone_ref()
        }
    }
}