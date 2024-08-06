use std::{io::{Error, ErrorKind}, sync::mpsc::Sender, thread::sleep, time::Duration};

use crate::{apps::sist_dron::calculations::{calculate_direction, calculate_distance}, logging::string_logger::StringLogger};

use super::{data::Data, dron_current_info::DronCurrentInfo, dron_state::DronState, sist_dron_properties::SistDronProperties};

#[derive(Debug)]
pub struct BatteryManager {
    current_data: Data,
    dron_properties: SistDronProperties,
    logger: StringLogger,
    ci_tx: Sender<DronCurrentInfo>,
}

impl BatteryManager {

    pub fn new(current_data: Data, dron_properties: SistDronProperties, logger: StringLogger, ci_tx: Sender<DronCurrentInfo>) -> Self {
        Self { current_data, dron_properties, logger, ci_tx }
    }

    pub fn run(&mut self) {
        loop {
            sleep(Duration::from_secs(5));
            
            //Actualizar batería
            let _ = self.decrement_and_check_battery_lvl();
        }
    }

    fn decrement_and_check_battery_lvl(&mut self) -> Result<(), Error> {
        let maintanence_position;
        let should_go_to_maintanence: bool;

        if let Ok(mut ci) = self.current_data.current_info.lock() {
            //decrementa la bateria
            let min_battery = self.dron_properties.get_min_operational_battery_lvl(); //20
            should_go_to_maintanence = ci.decrement_and_check_battery_lvl(min_battery); //inc=None
            maintanence_position = self.dron_properties.get_mantainance_position();
        //obelisco
        } else {
            return Err(Error::new(
                ErrorKind::Other,
                "Error al tomar lock de current info.",
            ));
        }

        if should_go_to_maintanence {
            self.logger
                .log("Batería baja, debo ir a mantenimiento.".to_string());
            // Se determina a qué posición volver después de cargarse
            let (position_to_go, state_to_set) = if self.current_data.get_state()? == DronState::ManagingIncident
            {
                (self.current_data.get_current_position()?, DronState::ManagingIncident)
            } else {
                (
                    self.dron_properties.get_range_center_position(),
                    DronState::ExpectingToRecvIncident,
                )
            };
            // Vuela a mantenimiento
            self.current_data.set_state(DronState::Mantainance, true)?;

            self.fly_to_mantainance(maintanence_position, true)?;
            sleep(Duration::from_secs(3));

            self.logger.log("Recargando batería al 100%.".to_string());
            self.recharge_battery()?; // podría llamarse recharge battery.

            // Vuelve a la posición correspondiente
            self.fly_to_mantainance(position_to_go, true)?;
            self.current_data.set_state(state_to_set, true)?;
        }
        Ok(())
    }

    fn fly_to_mantainance(
        &mut self,
        destination: (f64, f64),
        flag_maintanance: bool,
    ) -> Result<(), Error> {
        let origin = self.current_data.get_current_position()?;
        let dir = calculate_direction(origin, destination);
        println!("Fly_to: volando"); // se puede borrar
        self.logger.log(format!(
            "Fly_to: dir: {:?}, vel: {}",
            dir,
            self.dron_properties.get_speed()
        ));

        // self.current_data.set_state(DronState::Flying, flag_maintanance)?; // diferencia en caso mantenimiento
        self.current_data.set_flying_info_values(dir, self.dron_properties.get_speed(), flag_maintanance)?;

        let mut current_pos = origin;
        let threshold = 0.001; // Define un umbral adecuado para tu aplicación
        while calculate_distance(current_pos, destination) > threshold {
            current_pos = self.current_data.increment_current_position_in(dir, flag_maintanance)?;

            // Simular el vuelo, el dron se desplaza
            let a = 300; // aux
            sleep(Duration::from_micros(a));
            self.logger.log(format!(
                "   incrementada la posición actual: {:?}",
                self.current_data.get_current_position()
            ));

            // Publica
            self.publish_current_info()?;
        }

        // Salió del while porque está a muy poca distancia del destino. Hace ahora el paso final.
        self.current_data.set_current_position(destination)?;

        // Al llegar, el dron ya no se encuentra en desplazamiento.
        self.current_data.unset_flying_info_values()?;
        self.logger.log(format!(
            "   llegué a destino: {:?}",
            self.current_data.get_current_position()
        ));

        // Llegue a destino entonces debo cambiar a estado --> Manejando Incidente
        self.current_data.set_state(DronState::ManagingIncident, true)?;

        // Publica
        self.publish_current_info()?;

        println!("Fin vuelo."); // se podría borrar
        self.logger.log("Fin vuelo.".to_string());

        Ok(())
    }

    fn recharge_battery(&mut self) -> Result<(), Error> {
        self.current_data.set_battery_lvl(self.dron_properties.get_max_battery_lvl())?;
        Ok(())
    }

    /// Envía la current_info por un channel para que la parte receptora le haga publish.
    fn publish_current_info(&self) -> Result<(), Error> {
        let ci = self.current_data.get_current_info()?;
        if let Err(e) = self.ci_tx.send(ci) {
            println!("Error al enviar current_info para ser publicada: {:?}", e);
            self.logger.log(format!("Error al enviar current_info para ser publicada: {:?}.", e));
        }
        Ok(())
    }

}