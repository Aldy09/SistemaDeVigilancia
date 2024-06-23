use std::io::{Error, ErrorKind};

use super::super::properties::Properties;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SistDronProperties {
    max_battery_lvl: u8,
    min_operational_battery_lvl: u8,
    range: u8,
    stay_at_inc_time: u8, // Tiempo a permanencer en la ubicación del incidente, desde la llegada, en segundos.
    // Range center, porque un dron se mueve, al terminar de atender incidente vuelve a este range center
    range_center_lat: f64, // Aux: #ToDo: Capaz es mejor tener una Posicion, para no tener mil f64s sueltos []
    range_center_lon: f64,
    // Posicion de la central, para volver a cargarse la batería cuando se alcanza el min_operational_battery_lvl
    mantainance_lat: f64,
    mantainance_lon: f64,
    // Velocidad de vuelo, en km/h
    speed: f64,
}

impl SistDronProperties {
    pub fn new(properties_file: &str) -> Result<Self, Error> {
        // Cargamos todas las properties (constantes) del archivo, a este global_properties que es genérico
        let global_properties = Properties::new(properties_file)?;

        // Y ahora buscamos las properties específicas que usará el dron, y las cargamos a los campos de este struct
        let max_battery_lvl: u8;
        if let Some(prop) = global_properties.get("max_battery_lvl") {
            max_battery_lvl = prop
                .parse()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "max_battery_lvl"))?;
        } else {
            println!("No se encontró la propiedad 'max_battery_lvl");
            return Err(Error::new(
                ErrorKind::Other,
                "Falta propiedad max_battery_lvl.",
            ));
        }

        let min_operational_battery_lvl: u8;
        if let Some(prop) = global_properties.get("min_operational_battery_lvl") {
            min_operational_battery_lvl = prop
                .parse()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "min_operational_battery_lvl"))?;
        } else {
            println!("No se encontró la propiedad 'min_operational_battery_lvl");
            return Err(Error::new(ErrorKind::Other, "Falta propiedad sist dron."));
        }

        let range: u8;
        if let Some(prop) = global_properties.get("range") {
            range = prop
                .parse()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "range"))?;
        } else {
            println!("No se encontró la propiedad 'range");
            return Err(Error::new(ErrorKind::Other, "Falta propiedad sist dron."));
        }

        let stay_at_inc_time: u8;
        if let Some(prop) = global_properties.get("stay_at_inc_time") {
            stay_at_inc_time = prop
                .parse()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "stay_at_inc_time"))?;
        } else {
            println!("No se encontró la propiedad 'stay_at_inc_time");
            return Err(Error::new(ErrorKind::Other, "Falta propiedad sist dron."));
        }

        let range_center_lat: f64;
        if let Some(prop) = global_properties.get("range_center_lat") {
            range_center_lat = prop
                .parse()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "range_center_lat"))?;
        } else {
            println!("No se encontró la propiedad 'range_center_lat");
            return Err(Error::new(ErrorKind::Other, "Falta propiedad sist dron."));
        }

        let range_center_lon: f64;
        if let Some(prop) = global_properties.get("range_center_lon") {
            range_center_lon = prop
                .parse()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "range_center_lon"))?;
        } else {
            println!("No se encontró la propiedad 'range_center_lon");
            return Err(Error::new(ErrorKind::Other, "Falta propiedad sist dron."));
        }

        let mantainance_lat: f64;
        if let Some(prop) = global_properties.get("mantainance_lat") {
            mantainance_lat = prop
                .parse()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "mantainance_lat"))?;
        } else {
            println!("No se encontró la propiedad 'mantainance_lat");
            return Err(Error::new(ErrorKind::Other, "Falta propiedad sist dron."));
        }

        let mantainance_lon: f64;
        if let Some(prop) = global_properties.get("mantainance_lon") {
            mantainance_lon = prop
                .parse()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "mantainance_lon"))?;
        } else {
            println!("No se encontró la propiedad 'mantainance_lon");
            return Err(Error::new(ErrorKind::Other, "Falta propiedad sist dron."));
        }

        let speed: f64;
        if let Some(prop) = global_properties.get("speed") {
            speed = prop
                .parse()
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "speed"))?;
        } else {
            println!("No se encontró la propiedad 'speed");
            return Err(Error::new(ErrorKind::Other, "Falta propiedad sist dron."));
        }

        Ok(Self {
            max_battery_lvl,
            min_operational_battery_lvl,
            range,
            stay_at_inc_time,

            range_center_lat,
            range_center_lon,

            mantainance_lat,
            mantainance_lon,

            speed,
        })
    }

    /// Devuelve latitud y longitud del centro del rango, a la que volverá el dron luego de terminar de resolver un incidente
    pub fn get_range_center_position(&self) -> (f64, f64) {
        (self.range_center_lat, self.range_center_lon)
    }

    /// Devuelve el nivel mínimo de batería para poder funcionar
    pub fn get_min_operational_battery_lvl(&self) -> u8 {
        self.min_operational_battery_lvl
    }

    /// Devuelve el rango, utilizado para evaluar si atender o no incidentes
    pub fn get_range(&self) -> f64 {
        self.range as f64
    }

    /// Devuelve latitud y longitud del lugar de Mantenimiento, al que irá para recargar su batería
    pub fn get_mantainance_position(&self) -> (f64, f64) {
        (self.mantainance_lat, self.mantainance_lon)
    }

    /// Devuelve la velocidad de vuelo del dron
    pub fn get_speed(&self) -> f64 {
        self.speed
    }
}
