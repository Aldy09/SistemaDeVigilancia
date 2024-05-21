use std::io::{Error, ErrorKind};

use super::properties::Properties;

#[derive(Debug)]
pub struct SistCamsMQTTProperties {
    pub ip: String,
    pub port: i32,
    pub publish_interval: u64,
}

impl SistCamsMQTTProperties {
    pub fn new(global_properties: Properties) -> Result<Self, Error> {

        let ip: String;
        if let Some(prop) = global_properties.get("ip-server-mqtt") {
            ip = prop.to_owned();
        } else {
            println!("No se encontró la propiedad 'ip-server-mqtt");
            return Err(Error::new(ErrorKind::Other, "Falta propiedad sist cams mqtt."));
        }
        
        let port: i32;
        if let Some(prop) = global_properties.get("port-server-mqtt") {
            if let Ok(parsed_port) = prop.parse::<i32>() {
                port = parsed_port;
            } else {
                println!("Error al parsear 'port-server-mqtt");
                return Err(Error::new(ErrorKind::Other, "Error en propiedades sist cams mqtt."));
            }
        } else {
            println!("No se encontró la propiedad 'port-server-mqtt");
            return Err(Error::new(ErrorKind::Other, "Falta propiedad sist cams mqtt."));
        }

        let publish_interval: u64;
        if let Some(prop) = global_properties.get("publish-interval-mqtt") {
            if let Ok(parsed_interval) = prop.parse::<u64>() {
                publish_interval = parsed_interval;
            } else {
                println!("Error al parsear 'publish-interval-mqtt");
                return Err(Error::new(ErrorKind::Other, "Error en propiedades sist cams mqtt."));
            }
        } else {
            println!("No se encontró la propiedad 'publish-interval-mqtt");
            return Err(Error::new(ErrorKind::Other, "Falta propiedad sist cams mqtt."));
        }

        Ok(Self {
            ip,
            port,
            publish_interval,
        })

    }
}
