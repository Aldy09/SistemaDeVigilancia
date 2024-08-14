use std::{
    collections::{HashMap, VecDeque},
    io::{Error, ErrorKind},
    sync::{mpsc::{self, Sender}, Arc, Mutex}, thread::{self, sleep}, time::Duration,
};

use crate::{
    apps::{
        apps_mqtt_topics::AppsMqttTopics,
        incident_data::{
            incident::Incident, incident_info::IncidentInfo, incident_state::IncidentState,
        }, sist_dron::calculations::{calculate_direction, calculate_distance},
    },
    logging::string_logger::StringLogger,
    mqtt::messages::publish_message::PublishMessage,
};

use super::{
    data::Data, dron_current_info::DronCurrentInfo, dron_state::DronState,
    sist_dron_properties::SistDronProperties,
};

/// Componente encargado de manejar la lógica de procesamiento de incidentes de cada Dron.
#[derive(Debug)]
pub struct DronLogic {
    current_data: Data,
    dron_properties: SistDronProperties,
    logger: StringLogger,
    drone_distances_by_incident: DistancesType, // ya es arc mutex.
    ci_tx: Sender<DronCurrentInfo>,
    active_incs: Arc<Mutex<VecDeque<(IncidentInfo, Incident, u8)>>>, // el u8 es un contador de cuántos drones recibí que ya están yendo hacia ese inc.
}

type DistancesType = Arc<Mutex<HashMap<IncidentInfo, ((f64, f64), Vec<(u8, f64)>)>>>; // (inc_info, ( (inc_pos),(dron_id, distance_to_incident)) )

impl DronLogic {
    /// Crea un DronLogic.
    pub fn new(
        current_data: Data,
        dron_properties: SistDronProperties,
        logger: StringLogger,
        distances: DistancesType,
        ci_tx: Sender<DronCurrentInfo>,
    ) -> Self {
        Self {
            current_data,
            dron_properties,
            logger,
            drone_distances_by_incident: distances,
            ci_tx,
            active_incs: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn clone_ref(&self) -> Self {
        Self {
            current_data: self.current_data.clone_ref(),
            dron_properties: self.dron_properties,
            logger: self.logger.clone_ref(),
            drone_distances_by_incident: self.drone_distances_by_incident.clone(),
            ci_tx: self.ci_tx.clone(),
            active_incs: self.active_incs.clone(),
        }
    }

    /// Recibe un mensaje de los topics a los que se suscribió, y lo procesa.
    pub fn process_recvd_msg(
        &mut self,
        msg: PublishMessage,
        process_inc_tx: mpsc::Sender<()>,
    ) -> Result<(), Error> {
        let topic = msg.get_topic();
        let enum_topic = AppsMqttTopics::topic_from_str(topic.as_str())?;
        match enum_topic {
            AppsMqttTopics::IncidentTopic => self.process_valid_inc(msg.get_payload(), process_inc_tx),
            AppsMqttTopics::DronTopic => {
                let received_ci = DronCurrentInfo::from_bytes(msg.get_payload())?;
                let not_myself = self.current_data.get_id()? != received_ci.get_id();
                let recvd_dron_is_not_flying = received_ci.get_state() != DronState::Flying;
                let recvd_dron_is_not_managing_incident =
                    received_ci.get_state() != DronState::ManagingIncident;

                let recvd_dron_is_analyzing_if_should_move = received_ci.get_state() == DronState::RespondingToIncident;
                let recvd_dron_must_move = received_ci.get_state() == DronState::MustRespondToIncident;
                
                // Si la current_info recibida es de mi propio publish, no me interesa compararme conmigo mismo.
                // Si el current_info recibida es de un dron que está volando, tampoco me interesa, esos publish serán para sistema de moniteo.
                // Si el current_info recibida es de un dron que está en la ubicación de un incidente, tampoco me interesa, esos publish serán para sistema de moniteo.
                if not_myself {
                  
                  if recvd_dron_is_not_flying && recvd_dron_is_not_managing_incident {
                    if recvd_dron_is_analyzing_if_should_move {
                        self.process_valid_dron(received_ci)?;
                    }

                  } else if recvd_dron_must_move {
                    self.remove_from_active_incs_if_two_drones_already_flying(received_ci)?;
                  }                                

                }
                Ok(())
            }
            _ => Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "Topic no conocido",
            )),
        }
    }

    pub fn listen_for_and_process_new_active_incident(&mut self, rx: mpsc::Receiver<()>) -> Result<(), Error> {        
        for _ in rx {
            // Desencolo un incidente activo para procesarlo
            // Escucha por rx, for escucha algo por rx, hace esto:
            if self.current_data.get_state()? == DronState::ExpectingToRecvIncident {
                if let Some((_inc_info, inc, _dron_amount)) = self.pop_from_active_incs()? {
                    println!("DEBUG QUEUE: desacolé, voy a procesar el inc: {:?}", inc.get_source());
                    self.logger.log(format!("DEBUG QUEUE: desacolé, voy a procesar el inc: {:?}", inc.get_source()));
                    // Manda a ejecutar. Si falla no quiero cortar el loop, solo lo loggueo.
                    if let Err(e) = self.manage_and_check_incident(&inc) {
                        println!("DEBUG QUEUE: error en manage para inc: {:?}, {:?}", inc.get_source(), e);
                        self.logger.log(format!("DEBUG QUEUE: error en manage para inc: {:?}, {:?}", inc.get_source(), e));
                    }
                }
            }
        }

        Ok(())        
    }

    /// Recibe un incidente, analiza si está o no resuelto y actúa acorde.
    fn process_valid_inc(
        &mut self,
        payload: Vec<u8>,
        process_inc_tx: mpsc::Sender<()>,
    ) -> Result<(), Error> {
        let inc = Incident::from_bytes(payload)?;

        match *inc.get_state() {
            IncidentState::ActiveIncident => {
                // Encolo el inc activo recibido
                self.push_to_active_incs(&inc)?;
                // Se agrega la info del inc encolado, al distances, para que se haga el cálculo de las distancias para él tambiém
                self.add_incident_to_hashmap(&inc)?;
                // Al incio, y si recibe un inc estando en su pos inicial, va a estar en estado Expecting
                // Aviso al otro hilo que se puede desacolar y procesar el incidente activo
                let _ = process_inc_tx.send(());
                println!("DEBUG QUEUE: encolado el inc: {:?}", inc.get_source());
                self.logger.log(format!("DEBUG QUEUE: encolado el inc: {:?}", inc.get_source()));

                // // Desencolo un incidente activo para procesarlo
                // // Escucha por rx, for escucha algo por rx, hace esto:
                // if self.current_data.get_state()? == DronState::ExpectingToRecvIncident {
                //     if let Some((_inc_info, inc, _dron_amount)) = self.pop_from_active_incs()? {
                //         println!("DEBUG QUEUE: desacolé, voy a procesar el inc: {:?}", inc.get_source());
                //         self.logger.log(format!("DEBUG QUEUE: desacolé, voy a procesar el inc: {:?}", inc.get_source()));
                //         return self.manage_and_check_incident(&inc)
                //     }
                // }
                
            }
            IncidentState::ResolvedIncident => {
                self.go_back_if_my_inc_was_resolved(&inc)?;
                // Remuevo de el incidente resuelto de la queue de incs a procesar
                self.remove_from_active_incs(inc.get_info())?;
                // Aviso que ya se puede procesar el siguiente incidente activo encolado
                let _ = process_inc_tx.send(());
                println!("DEBUG QUEUE: se resolvió el inc: {:?}, enviando señal", inc.get_source());
                self.logger.log(format!("DEBUG QUEUE: se resolvió el inc: {:?}, enviando señal", inc.get_source()));


            }
        }

        Ok(())
    }

    fn manage_and_check_incident(&mut self, inc: &Incident) -> Result<(), Error> {
        match self.manage_incident(inc) {
            // Si la función termina con éxito, se devuelve ok.
            Ok(_) => Ok(()),
            // Si la función termina de procesar el incidente con error, hay que ver de qué tipo es el eroor
            Err(e) => {
                // Si fue de este tipo, éste es el caso en que la función dejó de procesar el incidente e hizo
                // return por ser interrumpida por poca batería y tener que volar a mantenimiento.
                // No es un error real, solo es una interrupción en el flujo de ejecución por ir a mantenimiento.
                if e.kind() == ErrorKind::InvalidData {
                    self.logger.log(format!(
                        "Se interrumpe procesamiento de inc {:?} para ir a mantenimiento.",
                        inc.get_info()
                    ));
                    Ok(())
                // Caso contrario sí fue un error real, y se devuelve.
                } else {
                    Err(e)
                }
            }
        }
    }

    fn push_to_active_incs(&mut self, inc: &Incident) -> Result<(), Error> {
        if let Ok(mut queue) = self.active_incs.lock(){
            queue.push_back((inc.get_info(), inc.clone(), 0));
            return Ok(());
        } 
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de active_incs.",
        ))

    }

    /// Hace pop de la estructure de incidentes activos a manejar, si la misma está vacía devuelve Ok(None).
    /// Y devuelve error si no se pudo tomar el lock.
    fn pop_from_active_incs(&mut self) -> Result<Option<(IncidentInfo, Incident, u8)>, Error>   {
        if let Ok(mut queue) = self.active_incs.lock(){
            return Ok(queue.pop_front());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de active_incs.",
        ))
    }

    fn remove_from_active_incs(&mut self, inc_info: IncidentInfo) -> Result<(), Error> {
        if let Ok(mut queue) = self.active_incs.lock(){
            if let Some(pos) = queue.iter().position(|(info, _, _)| *info == inc_info) {
                queue.remove(pos);
            }
            return Ok(());
        } 
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de active_incs.",
        ))        
    }

    /// Actualiza el contador de drones que ya están volando hacia el incidente del `ci` del dron recibido,
    /// y si el mismo ya vale 2, elimina el incidente de los `active_incs` para que luego ya no sea procesado.
    fn remove_from_active_incs_if_two_drones_already_flying(&mut self, ci: DronCurrentInfo) -> Result<(), Error> {
        // Obtiene el inc al que el dron recibido va a volar.
        if let Some(inc_info) = ci.get_inc_id_to_resolve() {
            if let Ok(mut queue) = self.active_incs.lock(){
                // Encuentra la posición del elemento (incidente) en la queue, y obtiene el elemento
                if let Some(pos) = queue.iter().position(|(info, _, _)| *info == inc_info) {
                    if let Some((_, _, mut amount_of_flying_drones)) = queue.get_mut(pos) {
                        // Suma uno al contador de drones que ya están volando hacia el inc
                        amount_of_flying_drones += 1;
                        // Si la cantidad vale 2, lo remuevo
                        if amount_of_flying_drones == 2 {
                            queue.remove(pos);
                        }
                    }
                }
                return Ok(());
            }
            return Err(Error::new(
                ErrorKind::Other,
                "Error al tomar lock de active_incs.",
            ));
        }

        Err(Error::new(
            ErrorKind::Other,
            "Error current_info recibido con estado e inc_info inválidos.",
        ))        
    }

    /// Por cada dron recibido si tenemos un incidente en comun se actualiza el hashmap con la menor distancia al incidente entre los drones (self_distance y recibido_distance).
    fn process_valid_dron(&self, received_dron: DronCurrentInfo) -> Result<(), Error> {
        // Obtengo el ID del incidente que el dron recibido está atendiendo
        if let Some(inc_info) = received_dron.get_inc_id_to_resolve() {
            if let Ok(mut distances) = self.drone_distances_by_incident.lock() {
                // Si el incidente ya está en el hashmap, agrego la menor distancia al incidente entre los dos drones. Si no, lo ignoro porque la rama "topic inc" no lo marco como de interés.
                if let Some((incident_position, candidate_drones)) = distances.get_mut(&inc_info) {
                    let received_dron_distance = received_dron.get_distance_to(*incident_position);

                    let self_distance = self.current_data.get_distance_to(*incident_position)?;

                    // Agrego al vector la menor distancia entre los dos drones al incidente
                    if self_distance <= received_dron_distance {
                        candidate_drones.push((self.current_data.get_id()?, self_distance));
                    } else {
                        candidate_drones.push((received_dron.get_id(), received_dron_distance));
                    }
                }
            }
        }

        Ok(())
    }

    fn decide_if_should_move_to_incident(
        &self,
        incident: &Incident,
    ) -> Result<bool, Error> {
        let mut should_move = false;

        //eSTE THREAD ES NECESARI. NO QUITAR
        thread::sleep(Duration::from_millis(3500)); // Aux Probando
        if let Ok(mut distances) = self.drone_distances_by_incident.lock() {
            if let Some((_incident_position, candidate_drones)) =
                distances.get_mut(&incident.get_info())
            {
                // Ordenar por el valor f64 de la tupla, de menor a mayor
                candidate_drones.sort_by(|a, b| a.1.total_cmp(&b.1));

                // Seleccionar los primeros dos elementos después de ordenar
                let closest_two_drones: Vec<u8> =
                    candidate_drones.iter().take(2).map(|&(id, _)| id).collect();

                // Si el id del dron actual está en la lista de los dos más cercanos, entonces se mueve
                should_move = closest_two_drones.contains(&self.current_data.get_id()?);
                self.logger.log(format!(
                    "Lado topic dron, evaluando distancias, debería moverme: {}",
                    should_move
                ));

                // Si está vacío, no se recibió aviso de un dron más cercano, entonces voy yo
                if closest_two_drones.is_empty() || closest_two_drones.len() == 1 {
                    should_move = true; // ()
                    self.logger.log(format!("Lado topic dron, evaluando distancias, debería moverme porque no hay nadie más: {}", should_move));
                }
            } else {
                self.logger.log(format!(
                    "Lado topic dron, esta condición no debería darse. Debería moverme: {}",
                    should_move
                ));
            }
        }

        Ok(should_move)
    }

    /// Publica su estado, y analiza condiciones para desplazarse.
    fn manage_incident(
        &mut self,
        inc_id: &Incident,
    ) -> Result<(), Error> {
        let event = format!("Recibido inc activo de id: {}", inc_id.get_id()); // se puede borrar
        println!("{:?}", event); // se puede borrar
        self.logger
            .log(format!("Recibido inc activo de id: {}", inc_id.get_id()));

        // Analizar condiciones para saber si se desplazará a la pos del incidente
        //  - batería es mayor al nivel bateria minima
        let batery_lvl = self.current_data.get_battery_lvl()?;
        let enough_battery = batery_lvl >= self.dron_properties.get_min_operational_battery_lvl();
        //  - inc.pos dentro del rango
        let (inc_lat, inc_lon) = inc_id.get_position();
        let inc_in_range =
            self.is_within_range_from_self(inc_lat, inc_lon, self.dron_properties.get_range());

        if enough_battery {
            if inc_in_range {
                println!(
                    "  está en rango, evaluando si desplazarme a inc {}",
                    inc_id.get_id()
                ); // se puede borrar
                self.logger.log(format!(
                    "  está en rango, evaluando si desplazarme a inc {}",
                    inc_id.get_id()
                ));
                self.current_data.set_inc_id_to_resolve(inc_id.get_info())?; //
                self.add_incident_to_hashmap(&inc_id)?;

                self.current_data
                    .set_state(DronState::RespondingToIncident, false)?;

                // Publica su estado (su current info) para que otros drones vean la condición b, y monitoreo lo muestre en mapa
                self.publish_current_info()?;

                let should_move =
                    self.decide_if_should_move_to_incident(&inc_id)?;
                println!("   debería ir al incidente según cercanía: {}", should_move); // se puede borrar
                self.logger.log(format!(
                    "   debería ir al incidente según cercanía: {}",
                    should_move
                ));
                if should_move {
                    // Setea estado y avisa que quedó como ganador y se moverá al incidente
                    self.current_data.set_state(DronState::MustRespondToIncident, false)?;
                    self.publish_current_info()?;

                    // Volar hasta la posición del incidente
                    let destination = inc_id.get_position();
                    self.fly_to(destination)?;
                    self.remove_incident_from_hashmap(&inc_id)?;
                }
            } else {
                println!("   el inc No está en mi rango."); // se puede borrar
                self.logger
                    .log(format!("  el inc {} No está en rango.", inc_id.get_id()));
            }
        } else {
            // No tiene suficiente batería, por lo que debe ir a mantenimiento a recargarse
            self.current_data.set_state(DronState::Mantainance, false)?;

            // Volar a la posición de Mantenimiento
            let destination = self.dron_properties.get_range_center_position();
            self.fly_to(destination)?;
        }

        Ok(())
    }

    /// Calcula si se encuentra las coordenadas pasadas se encuentran dentro de su rango.
    fn is_within_range_from_self(&self, latitude: f64, longitude: f64, range: f64) -> bool {
        let (center_lat, center_lon) = self.dron_properties.get_range_center_position();
        let lat_dist = center_lat - latitude;
        let long_dist = center_lon - longitude;
        let rad = f64::sqrt(lat_dist.powi(2) + long_dist.powi(2));

        // Ajuste para aprox dos manzanas en diagonal
        let adjusted_range = range / 1000.0; // hay que modificar el range de las cámaras, ahora que son latitudes de verdad y no "3 4".
                                             //println!("Dio que la cuenta vale: {}, y adj_range vale: {}. Era rango: {}", rad, adjusted_range, range); // debug []

        rad <= (adjusted_range)
    }

    /// Analiza si el incidente que se resolvió fue el que el dron self estaba atendiendo.
    /// Si sí, entonces vuelve al centro de su rango (su posición inicial) y actualiza su estado.
    /// Si no, lo ignoro porque no era el incidente que este dron estaba atendiendo.
    fn go_back_if_my_inc_was_resolved(
        &mut self,
        inc: &Incident,
    ) -> Result<(), Error> {
        self.logger
            .log(format!("Recibido inc resuelto de id: {}", inc.get_id()));

        if let Some(my_inc_id) = self.current_data.get_inc_id_to_resolve()? {
            if inc.get_info() == my_inc_id {
                let event = format!(
                    "Recibido inc resuelto de id: {}, volviendo a posición inicial.",
                    inc.get_id()
                ); // se puede borrar
                println!("{:?}", event); // se puede borrar

                self.logger.log(format!(
                    "Recibido inc resuelto de id: {}, volviendo a posición inicial",
                    inc.get_id()
                ));
                self.current_data.unset_inc_id_to_resolve()?; // [lo he subido una línea] [] aux
                self.go_back_to_range_center_position()?;
                
            }
        }

        Ok(())
    }

    /// Vuelve al centro de su rango (su posición inicial), y una vez que llega actualiza su estado
    /// para continuar escuchando incidentes.
    fn go_back_to_range_center_position(
        &mut self,
    ) -> Result<(), Error> {
        // Volver, volar al range center
        let destination = self.dron_properties.get_range_center_position();
        self.fly_to(destination)?;

        // Una vez que llegué: Setear estado a nuevamente recibir incidentes
        self.current_data
            .set_state(DronState::ExpectingToRecvIncident, false)?;

        Ok(())
    }

    fn fly_to(
        &mut self,
        destination: (f64, f64),
    ) -> Result<(), Error> {
        let origin = self.current_data.get_current_position()?;
        let dir = calculate_direction(origin, destination);
        println!("Fly_to: volando"); // se puede borrar
        self.logger.log(format!(
            "Fly_to: dir: {:?}, vel: {}",
            dir,
            self.dron_properties.get_speed()
        ));

        self.current_data.set_state(DronState::Flying, false)?;
        self.current_data
            .set_flying_info_values(dir, self.dron_properties.get_speed(), false)?;
        let mut current_pos = origin;
        let threshold = 0.001; //
        while calculate_distance(current_pos, destination) > threshold {
            current_pos = self
                .current_data
                .increment_current_position_in(dir, false)?;

            // Simula el vuelo, el dron se desplaza
            let a = 4/5; // aux
            sleep(Duration::from_secs(a));
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
        self.current_data
            .set_state(DronState::ManagingIncident, false)?;

        // Publica
        self.publish_current_info()?;

        println!("Fin vuelo."); // se podría borrar
        self.logger.log("Fin vuelo.".to_string());

        Ok(())
    }

    fn add_incident_to_hashmap(&self, inc: &Incident) -> Result<(), Error> {
        if let Ok(mut distances) = self.drone_distances_by_incident.lock() {
            distances.insert(inc.get_info(), (inc.get_position(), Vec::new()));
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de drone_distances_by_incident.",
        ))
    }

    fn remove_incident_from_hashmap(&self, inc: &Incident) -> Result<(), Error> {
        if let Ok(mut distances) = self.drone_distances_by_incident.lock() {
            distances.remove(&inc.get_info());
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de drone_distances_by_incident.",
        ))
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
