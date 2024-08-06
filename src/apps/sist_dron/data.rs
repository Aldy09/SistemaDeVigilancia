use std::{io::{Error, ErrorKind}, sync::{Arc, Mutex}};

use crate::apps::incident_data::incident_info::IncidentInfo;

use super::{dron_current_info::DronCurrentInfo, dron_flying_info::DronFlyingInfo, dron_state::DronState};

#[derive(Debug)]
pub struct Data {
    pub current_info: Arc<Mutex<DronCurrentInfo>>, // Aux: lo hago pub solo por un momento, lo usa solamente el battery en una línea, dsp lo ponemos privado otra vez. [].
}

impl Data {
    pub fn new(current_info: Arc<Mutex<DronCurrentInfo>>) -> Self {
        Self { current_info }
    }

    /// Toma lock y obtiene el id del dron.
    pub fn get_id(&self) -> Result<u8, Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.get_id());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    /// Toma lock y obteiene el estado en que se encuentra el dron.
    pub fn get_state(&self) -> Result<DronState, Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.get_state());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }
    /// Toma lock y establece el estado en que se encuentra el dron.
    /// El flag de mantenimiento indica si quien lo llama es o no el módulo de mantenimiento,
    /// y se utiliza para otorgar permisos.
    pub fn set_state(&self, new_state: DronState, flag_maintanance: bool) -> Result<(), Error> {
        if let Ok(mut ci) = self.current_info.lock() {
            let is_mantainance_set = flag_maintanance;
            let is_not_maintainance_set =
                ci.get_state() != DronState::Mantainance && !flag_maintanance;
            if is_mantainance_set || is_not_maintainance_set {
                ci.set_state(new_state);
                return Ok(());
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Error al tomar lock de current info.",
                ));
            };
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    /// Establece como `flying_info` a la dirección recibida, y a la velocidad leída del archivo de configuración.
    /// El flag de mantenimiento indica si quien lo llama es o no el módulo de mantenimiento,
    /// y se utiliza para otorgar permisos.
    pub fn set_flying_info_values(
        &mut self,
        dir: (f64, f64),
        speed: f64,
        flag_maintanance: bool,
    ) -> Result<(), Error> {
        let is_mantainance_set = flag_maintanance;
        let is_not_maintainance_set =
            self.get_state()? != DronState::Mantainance && !flag_maintanance;
        if is_mantainance_set || is_not_maintainance_set {
            let info = DronFlyingInfo::new(dir, speed);
            self.set_flying_info(info)?;
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::InvalidData,
                "Error al tomar lock de current info.",
            ))
        }
    }

    /// Establece `None` como `flying_info`, lo cual indica que el dron no está actualmente en desplazamiento.
    /// Toma lock en el proceso.
    pub fn unset_flying_info_values(&mut self) -> Result<(), Error> {
        if let Ok(mut ci) = self.current_info.lock() {
            ci.unset_flying_info();
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }
    /// Función interna, una vez manejados los permisos.
    fn set_flying_info(&self, info: DronFlyingInfo) -> Result<(), Error> {
        if let Ok(mut ci) = self.current_info.lock() {
            ci.set_flying_info(info);
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    /// Toma lock y devuelve su nivel de batería.
    pub fn get_battery_lvl(&self) -> Result<u8, Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.get_battery_lvl());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    /// Toma lock y establece su nivel de batería al recibido por parámetro.
    pub fn set_battery_lvl(&mut self, new_battery_level: u8) -> Result<(), Error> {
        if let Ok(mut ci) = self.current_info.lock() {
            ci.set_battery_lvl(new_battery_level);
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "Error al tomar lock de current info.",
            ))
        }
    }

    /// Toma lock y establece el inc id a resolver.
    pub fn set_inc_id_to_resolve(&self, inc_info: IncidentInfo) -> Result<(), Error> {
        if let Ok(mut ci) = self.current_info.lock() {
            ci.set_inc_id_to_resolve(inc_info);
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }
    /// Toma lock y borra el inc id a resolver.
    pub fn unset_inc_id_to_resolve(&self) -> Result<(), Error> {
        if let Ok(mut ci) = self.current_info.lock() {
            ci.unset_inc_id_to_resolve();
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }
    /// Toma lock y devuelve el inc_id a resolver.
    pub fn get_inc_id_to_resolve(&self) -> Result<Option<IncidentInfo>, Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.get_inc_id_to_resolve());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    /// Toma lock y obtiene la `current_position`, posición en la que el dron se encuentra actualmente.
    pub fn get_current_position(&self) -> Result<(f64, f64), Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.get_current_position());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }
    /// Toma lock, incrementa la `current_position` en la dirección recibida, y la devuelve actualizada.
    /// El flag de mantenimiento indica si quien llama a esta función es el módulo encargado del mantenimiento,
    /// y se utiliza para otorgar permisos.
    pub fn increment_current_position_in(
        &self,
        dir: (f64, f64),
        flag_maintanance: bool,
    ) -> Result<(f64, f64), Error> {
        if let Ok(mut ci) = self.current_info.lock() {
            let is_mantainance_set = flag_maintanance;
            let is_not_maintainance_set =
                ci.get_state() != DronState::Mantainance && !flag_maintanance;
            if is_mantainance_set || is_not_maintainance_set {
                Ok(ci.increment_current_position_in(dir))
            } else {
                Err(Error::new(
                    ErrorKind::InvalidData,
                    "Error al tomar lock de current info.",
                ))
            }
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "Error al tomar lock de current info.",
            ))
        }
    }
    /// Toma lock y establece la `current_position` en la recibida por parámetro.
    pub fn set_current_position(&self, new_position: (f64, f64)) -> Result<(), Error> {
        if let Ok(mut ci) = self.current_info.lock() {
            ci.set_current_position(new_position);
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    pub fn clone_ref(&self) -> Self {
        Self {
            current_info: self.current_info.clone(),
        }
    }
    
    /// Devuelve una copia del valor actual de la `current_info`. Utilizado para enviar por channel.
    pub fn get_current_info(&self) -> Result<DronCurrentInfo, Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.clone());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    // []
    pub fn get_distance_to(&self, destination: (f64, f64)) -> Result<f64, Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.get_distance_to(destination));
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

}