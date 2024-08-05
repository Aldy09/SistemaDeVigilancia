use std::{
    collections::HashMap,
    fs,
    io::{self, Error, ErrorKind},
    sync::{Arc, Mutex},
    thread::{self, sleep, JoinHandle},
    time::Duration,
};

use std::sync::mpsc::Receiver as MpscReceiver;

use crate::apps::{common_clients::there_are_no_more_publish_msgs, incident_data::{
    incident::Incident, incident_info::IncidentInfo, incident_state::IncidentState,
}, sist_dron::calculations::{calculate_direction, calculate_distance}};
use crate::apps::{
    apps_mqtt_topics::AppsMqttTopics, common_clients::join_all_threads,
    sist_dron::dron_state::DronState,
};
use crate::logging::string_logger::StringLogger;
use crate::mqtt::{client::mqtt_client::MQTTClient, messages::publish_message::PublishMessage};

use super::{
    data::Data, dron_current_info::DronCurrentInfo, sist_dron_properties::SistDronProperties
};

type DistancesType = Arc<Mutex<HashMap<IncidentInfo, ((f64, f64), Vec<(u8, f64)>)>>>; // (inc_info, ( (inc_pos),(dron_id, distance_to_incident)) )

/// Struct que representa a cada uno de los drones del sistema de vigilancia.
/// Al publicar en el topic `dron`, solamente el struct `DronCurrentInfo` es lo que interesa enviar,
/// ya que lo demás son constantes para el funcionamiento del Dron.
#[derive(Debug)]
pub struct Dron {
    data: Data,
    // El id y su posición y estado actuales se encuentran en el siguiente struct
    current_info: Arc<Mutex<DronCurrentInfo>>,

    // Constantes cargadas desde un arch de configuración
    dron_properties: SistDronProperties,

    logger: StringLogger,

    drone_distances_by_incident: DistancesType,
    qos: u8,
}

#[allow(dead_code)]
impl Dron {
    /// Dron se inicia con batería al 100%, desde la posición del range_center, con estado activo.
    pub fn new(id: u8, lat: f64, lon: f64, logger: StringLogger) -> Result<Self, Error> {
        let dron = Self::new_internal(id, lat, lon, logger)?;
        dron.logger.log(format!("Dron: Iniciado dron {:?}", id));

        Ok(dron)
    }

    fn spawn_for_update_battery(&self, mqtt_client: Arc<Mutex<MQTTClient>>) -> JoinHandle<()> {
        let mut self_child = self.clone_ref();
        thread::spawn(move ||
            //let battery_manager = BatteryManager::new(self_child.data);
            loop {
            sleep(Duration::from_secs(5));
            //Actualizar batería

            let _ = self_child.decrement_and_check_battery_lvl(&mqtt_client.clone());
        })
    }

    pub fn get_qos(&self) -> u8 {
        self.qos
    }

    pub fn get_current_info(&self) -> &Arc<Mutex<DronCurrentInfo>> {
        &self.current_info
    }

    pub fn spawn_threads(
        &mut self,
        mqtt_client: MQTTClient,
        mqtt_rx: MpscReceiver<PublishMessage>,
    ) -> Result<Vec<JoinHandle<()>>, Error> {
        let mut children: Vec<JoinHandle<()>> = vec![];
        let mqtt_client_sh = Arc::new(Mutex::new(mqtt_client));

        children.push(self.spawn_for_update_battery(mqtt_client_sh.clone()));

        self.subscribe_to_topics(Arc::clone(&mqtt_client_sh), mqtt_rx)?;

        Ok(children)
    }

    fn clone_ref(&self) -> Self {
        Self {
            data: self.data.clone_ref(),
            current_info: Arc::clone(&self.current_info),
            dron_properties: self.dron_properties,
            logger: self.logger.clone_ref(),
            drone_distances_by_incident: Arc::clone(&self.drone_distances_by_incident),
            qos: self.qos,
        }
    }

    /// Se suscribe a topics inc y dron, y lanza la recepción de mensajes y finalización.
    fn subscribe_to_topics(
        &mut self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        mqtt_rx: MpscReceiver<PublishMessage>,
    ) -> Result<(), Error> {
        self.subscribe_to_topic(&mqtt_client, AppsMqttTopics::IncidentTopic.to_str())?;
        self.subscribe_to_topic(&mqtt_client, AppsMqttTopics::DronTopic.to_str())?;
        self.receive_messages_from_subscribed_topics(&mqtt_client, mqtt_rx);

        Ok(())
    }

    /// Se suscribe al topic recibido.
    fn subscribe_to_topic(
        &self,
        mqtt_client: &Arc<Mutex<MQTTClient>>,
        topic: &str,
    ) -> Result<(), Error> {
        if let Ok(mut mqtt_client) = mqtt_client.lock() {
            let res_sub = mqtt_client.mqtt_subscribe(vec![(String::from(topic))]);
            match res_sub {
                Ok(_) => {
                    self.logger
                        .log(format!("Dron: Suscripto a topic: {}", topic));
                }
                Err(_) => {
                    return Err(Error::new(
                        std::io::ErrorKind::Other,
                        "Error al hacer un subscribe a topic",
                    ))
                }
            }
        }
        Ok(())
    }

    /// Recibe mensajes de los topics a los que se ha suscrito: inc y dron.
    /// (aux sist monitoreo actualiza el estado del incidente y hace publish a inc; dron hace publish a dron)
    /// Lanza un hilo por cada mensaje recibido, para procesarlo, y espera a sus hijos.
    fn receive_messages_from_subscribed_topics(
        &mut self,
        mqtt_client: &Arc<Mutex<MQTTClient>>,
        mqtt_rx: MpscReceiver<PublishMessage>,
    ) {
        let mut children = vec![];

        for publish_msg in mqtt_rx {
            self.logger
                .log(format!("Dron: Recibo mensaje Publish: {:?}", publish_msg));

            // Lanza un hilo para procesar el mensaje, y luego lo espera correctamente
            let handle_thread = self.spawn_process_recvd_msg_thread(publish_msg, mqtt_client);
            children.push(handle_thread);
        }
        there_are_no_more_publish_msgs(&self.logger);

        join_all_threads(children);
    }

    fn spawn_process_recvd_msg_thread(
        &self,
        msg: PublishMessage,
        mqtt_client: &Arc<Mutex<MQTTClient>>,
    ) -> JoinHandle<()> {
        let mut self_clone = self.clone_ref();
        let mqtt_client_clone = Arc::clone(mqtt_client);
        thread::spawn(move || {
            let _ = self_clone.process_recvd_msg(msg, &mqtt_client_clone);
        })
    }

    /// Recibe un mensaje de los topics a los que se suscribió, y lo procesa.
    fn process_recvd_msg(
        &mut self,
        msg: PublishMessage,
        mqtt_client: &Arc<Mutex<MQTTClient>>,
    ) -> Result<(), Error> {
        let topic = msg.get_topic();
        let enum_topic = AppsMqttTopics::topic_from_str(topic.as_str())?;
        match enum_topic {
            AppsMqttTopics::IncidentTopic => self.process_valid_inc(msg.get_payload(), mqtt_client),
            AppsMqttTopics::DronTopic => {
                let received_ci = DronCurrentInfo::from_bytes(msg.get_payload())?;
                let not_myself = self.data.get_id()? != received_ci.get_id();
                let recvd_dron_is_not_flying = received_ci.get_state() != DronState::Flying;
                let recvd_dron_is_not_managing_incident =
                    received_ci.get_state() != DronState::ManagingIncident;

                // Si la current_info recibida es de mi propio publish, no me interesa compararme conmigo mismo.
                // Si el current_info recibida es de un dron que está volando, tampoco me interesa, esos publish serán para sistema de moniteo.
                // Si el current_info recibida es de un dron que está en la ubicación de un incidente, tampoco me interesa, esos publish serán para sistema de moniteo.
                if not_myself && recvd_dron_is_not_flying && recvd_dron_is_not_managing_incident {
                    self.process_valid_dron(received_ci)?;
                }
                Ok(())
            }
            _ => Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "Topic no conocido",
            )),
        }
    }

    /// Recibe un incidente, analiza si está o no resuelto y actúa acorde.
    fn process_valid_inc(
        &mut self,
        payload: Vec<u8>,
        mqtt_client: &Arc<Mutex<MQTTClient>>,
    ) -> Result<(), Error> {
        let inc = Incident::from_bytes(payload)?;
        let inc_id = inc.get_id();

        match *inc.get_state() {
            IncidentState::ActiveIncident => {
                match self.manage_incident(inc, mqtt_client) {
                    // Si la función termina con éxito, se devuelve ok.
                    Ok(_) => Ok(()),
                    // Si la función termina de procesar el incidente con error, hay que ver de qué tipo es el eroor
                    Err(e) => {
                        // Si fue de este tipo, éste es el caso en que la función dejó de procesar el incidente e hizo
                        // return por ser interrumpida por poca batería y tener que volar a mantenimiento.
                        // No es un error real, solo es una interrupción en el flujo de ejecución por ir a mantenimiento.
                        if e.kind() == ErrorKind::InvalidData {
                            self.logger.log(format!(
                                "Se interrumpe procesamiento de inc {} para ir a mantenimiento.",
                                inc_id
                            ));
                            Ok(())
                        // Caso contrario sí fue un error real, y se devuelve.
                        } else {
                            Err(e)
                        }
                    }
                }
            }
            IncidentState::ResolvedIncident => {
                self.go_back_if_my_inc_was_resolved(inc, mqtt_client)?;
                Ok(())
            }
        }
    }

    /// Por cada dron recibido si tenemos un incidente en comun se actualiza el hashmap con la menor distancia al incidente entre los drones (self_distance y recibido_distance).
    fn process_valid_dron(&self, received_dron: DronCurrentInfo) -> Result<(), Error> {
        // Obtengo el ID del incidente que el dron recibido está atendiendo
        if let Some(inc_info) = received_dron.get_inc_id_to_resolve() {
            if let Ok(mut distances) = self.drone_distances_by_incident.lock() {
                // Si el incidente ya está en el hashmap, agrego la menor distancia al incidente entre los dos drones. Si no, lo ignoro porque la rama "topic inc" no lo marco como de interés.
                if let Some((incident_position, candidate_drones)) = distances.get_mut(&inc_info) {
                    let received_dron_distance = received_dron.get_distance_to(*incident_position);

                    let self_distance = self.get_distance_to(*incident_position)?;

                    // Agrego al vector la menor distancia entre los dos drones al incidente
                    if self_distance <= received_dron_distance {
                        candidate_drones.push((self.data.get_id()?, self_distance));
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
        _mqtt_client: Arc<Mutex<MQTTClient>>,
    ) -> Result<bool, Error> {
        let mut should_move = false;

        //thread::sleep(Duration::from_millis(500));
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
                should_move = closest_two_drones.contains(&self.data.get_id()?);
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
        inc_id: Incident,
        mqtt_client: &Arc<Mutex<MQTTClient>>,
    ) -> Result<(), Error> {
        let event = format!("Recibido inc activo de id: {}", inc_id.get_id()); // se puede borrar
        println!("{:?}", event); // se puede borrar
        self.logger
            .log(format!("Recibido inc activo de id: {}", inc_id.get_id()));

        // Analizar condiciones para saber si se desplazará a la pos del incidente
        //  - batería es mayor al nivel bateria minima
        let batery_lvl = self.data.get_battery_lvl()?;
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
                self.data.set_inc_id_to_resolve(inc_id.get_info())?; //
                self.add_incident_to_hashmap(&inc_id)?;

                self.data.set_state(DronState::RespondingToIncident, false)?;

                // Publica su estado (su current info) para que otros drones vean la condición b, y monitoreo lo muestre en mapa
                self.publish_current_info(mqtt_client)?;

                let should_move =
                    self.decide_if_should_move_to_incident(&inc_id, mqtt_client.clone())?;
                println!("   debería ir al incidente según cercanía: {}", should_move); // se puede borrar
                self.logger.log(format!(
                    "   debería ir al incidente según cercanía: {}",
                    should_move
                ));
                if should_move {
                    // Volar hasta la posición del incidente
                    let destination = inc_id.get_position();
                    self.fly_to(destination, mqtt_client)?;
                    self.remove_incident_to_hashmap(&inc_id)?;
                }
            } else {
                println!("   el inc No está en mi rango."); // se puede borrar
                self.logger
                    .log(format!("  el inc {} No está en rango.", inc_id.get_id()));
            }
        } else {
            // No tiene suficiente batería, por lo que debe ir a mantenimiento a recargarse
            self.data.set_state(DronState::Mantainance, false)?;

            // Volar a la posición de Mantenimiento
            let destination = self.dron_properties.get_range_center_position();
            self.fly_to(destination, mqtt_client)?;
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
        inc: Incident,
        mqtt_client: &Arc<Mutex<MQTTClient>>,
    ) -> Result<(), Error> {
        self.logger
            .log(format!("Recibido inc resuelto de id: {}", inc.get_id()));

        if let Some(my_inc_id) = self.data.get_inc_id_to_resolve()? {
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
                self.go_back_to_range_center_position(mqtt_client)?;
                self.data.unset_inc_id_to_resolve()?;
            }
        }

        Ok(())
    }

    /// Vuelve al centro de su rango (su posición inicial), y una vez que llega actualiza su estado
    /// para continuar escuchando incidentes.
    fn go_back_to_range_center_position(
        &mut self,
        mqtt_client: &Arc<Mutex<MQTTClient>>,
    ) -> Result<(), Error> {
        // Volver, volar al range center
        let destination = self.dron_properties.get_range_center_position();
        self.fly_to(destination, mqtt_client)?;

        // Una vez que llegué: Setear estado a nuevamente recibir incidentes
        self.data.set_state(DronState::ExpectingToRecvIncident, false)?;

        Ok(())
    }    

    fn fly_to_mantainance(
        &mut self,
        destination: (f64, f64),
        mqtt_client: &Arc<Mutex<MQTTClient>>,
        flag_maintanance: bool,
    ) -> Result<(), Error> {
        let origin = self.data.get_current_position()?;
        let dir = calculate_direction(origin, destination);
        println!("Fly_to: volando"); // se puede borrar
        self.logger.log(format!(
            "Fly_to: dir: {:?}, vel: {}",
            dir,
            self.dron_properties.get_speed()
        ));

        // self.data.set_state(DronState::Flying, flag_maintanance)?; // diferencia en caso mantenimiento
        self.data.set_flying_info_values(dir, flag_maintanance)?;

        let mut current_pos = origin;
        let threshold = 0.001; // Define un umbral adecuado para tu aplicación
        while calculate_distance(current_pos, destination) > threshold {
            current_pos = self.data.increment_current_position_in(dir, flag_maintanance)?;

            // Simular el vuelo, el dron se desplaza
            let a = 300; // aux
            sleep(Duration::from_micros(a));
            self.logger.log(format!(
                "   incrementada la posición actual: {:?}",
                self.data.get_current_position()
            ));

            // Publica
            self.publish_current_info(mqtt_client)?;
        }

        // Salió del while porque está a muy poca distancia del destino. Hace ahora el paso final.
        self.data.set_current_position(destination)?;

        // Al llegar, el dron ya no se encuentra en desplazamiento.
        self.data.unset_flying_info_values()?;
        self.logger.log(format!(
            "   llegué a destino: {:?}",
            self.data.get_current_position()
        ));

        // Llegue a destino entonces debo cambiar a estado --> Manejando Incidente
        self.data.set_state(DronState::ManagingIncident, true)?;

        // Publica
        self.publish_current_info(mqtt_client)?;

        println!("Fin vuelo."); // se podría borrar
        self.logger.log("Fin vuelo.".to_string());

        Ok(())
    }

    fn fly_to(
        &mut self,
        destination: (f64, f64),
        mqtt_client: &Arc<Mutex<MQTTClient>>,
    ) -> Result<(), Error> {
        let origin = self.data.get_current_position()?;
        let dir = calculate_direction(origin, destination);
        println!("Fly_to: volando"); // se puede borrar
        self.logger.log(format!(
            "Fly_to: dir: {:?}, vel: {}",
            dir,
            self.dron_properties.get_speed()
        ));

        self.data.set_state(DronState::Flying, false)?;
        self.data.set_flying_info_values(dir, false)?;
        let mut current_pos = origin;
        let threshold = 0.001; //
        while calculate_distance(current_pos, destination) > threshold {
            current_pos = self.data.increment_current_position_in(dir, false)?;

            // Simula el vuelo, el dron se desplaza
            let a = 300; // aux
            sleep(Duration::from_micros(a));
            self.logger.log(format!(
                "   incrementada la posición actual: {:?}",
                self.data.get_current_position()
            ));

            // Publica
            self.publish_current_info(mqtt_client)?;
        }

        // Salió del while porque está a muy poca distancia del destino. Hace ahora el paso final.
        self.data.set_current_position(destination)?;

        // Al llegar, el dron ya no se encuentra en desplazamiento.
        self.data.unset_flying_info_values()?;
        self.logger.log(format!(
            "   llegué a destino: {:?}",
            self.data.get_current_position()
        ));

        // Llegue a destino entonces debo cambiar a estado --> Manejando Incidente
        self.data.set_state(DronState::ManagingIncident, false)?;

        // Publica
        self.publish_current_info(mqtt_client)?;

        println!("Fin vuelo."); // se podría borrar
        self.logger.log("Fin vuelo.".to_string());

        Ok(())
    }

    /// Hace publish de su current info.
    /// Le servirá a otros drones para ver la condición de los dos drones más cercanos y a monitoreo para mostrarlo en mapa.
    fn publish_current_info(&self, mqtt_client: &Arc<Mutex<MQTTClient>>) -> Result<(), Error> {
        if let Ok(mut mqtt_client_l) = mqtt_client.lock() {
            if let Ok(ci) = &self.current_info.lock() {
                mqtt_client_l.mqtt_publish(
                    AppsMqttTopics::DronTopic.to_str(),
                    &ci.to_bytes(),
                    self.qos,
                )?;
            }
        };
        Ok(())
    }

    /// Establece como `flying_info` a la dirección recibida, y a la velocidad leída del archivo de configuración.
    

    /// Establece `None` como `flying_info`, lo cual indica que el dron no está actualmente en desplazamiento.

    //// Funciones que toman lock ////

    fn get_distance_to(&self, destination: (f64, f64)) -> Result<f64, Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.get_distance_to(destination));
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    fn leer_qos_desde_archivo(ruta_archivo: &str) -> Result<u8, io::Error> {
        let contenido = fs::read_to_string(ruta_archivo)?;
        let inicio = contenido.find("qos=").ok_or(io::Error::new(
            ErrorKind::NotFound,
            "No se encontró la etiqueta 'qos='",
        ))?;
        let valor_qos = contenido[inicio + 4..].trim().parse::<u8>().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "El valor de QoS no es un número válido",
            )
        })?;
        Ok(valor_qos)
    }

    /// Dron se inicia con batería al 100%, desde la posición del range_center, con estado activo.
    /// Función utilizada para testear, no necesita broker address.
    fn new_internal(
        id: u8,
        initial_lat: f64,
        initial_lon: f64,
        logger: StringLogger,
    ) -> Result<Self, Error> {
        let qos = Dron::leer_qos_desde_archivo("src/apps/sist_dron/qos_dron.properties")?;
        // Se cargan las constantes desde archivo de config.
        let properties_file = "src/apps/sist_dron/sistema_dron.properties";
        let mut dron_properties = SistDronProperties::new(properties_file)?;
        let drone_distances_by_incident = Arc::new(Mutex::new(HashMap::new()));

        // Inicia desde el range_center, por lo cual tiene estado activo; y con batería al 100%.
        // Posicion inicial del dron
        dron_properties.set_range_center_position(initial_lat, initial_lon);

        let ci = DronCurrentInfo::new(
            id,
            initial_lat,
            initial_lon,
            100,
            DronState::ExpectingToRecvIncident,
        );

        logger.log(format!(
            "Dron {} creado en posición (lat, lon): {}, {}.",
            id, initial_lat, initial_lon
        ));
        let current_info = Arc::new(Mutex::new(ci));
        let dron = Dron {
            data: Data::new(current_info.clone(), dron_properties),
            current_info,
            dron_properties,
            logger,
            drone_distances_by_incident,
            qos,
        };

        Ok(dron)
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

    fn remove_incident_to_hashmap(&self, inc: &Incident) -> Result<(), Error> {
        if let Ok(mut distances) = self.drone_distances_by_incident.lock() {
            distances.remove(&inc.get_info());
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de drone_distances_by_incident.",
        ))
    }

    fn decrement_and_check_battery_lvl(
        &mut self,
        mqtt_client: &Arc<Mutex<MQTTClient>>,
    ) -> Result<(), Error> {
        let maintanence_position;
        let should_go_to_maintanence: bool;

        if let Ok(mut ci) = self.current_info.lock() {
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
            let (position_to_go, state_to_set) = if self.data.get_state()? == DronState::ManagingIncident
            {
                (self.data.get_current_position()?, DronState::ManagingIncident)
            } else {
                (
                    self.dron_properties.get_range_center_position(),
                    DronState::ExpectingToRecvIncident,
                )
            };
            // Vuela a mantenimiento
            self.data.set_state(DronState::Mantainance, true)?;

            self.fly_to_mantainance(maintanence_position, mqtt_client, true)?;
            sleep(Duration::from_secs(3));

            self.logger.log("Recargando batería al 100%.".to_string());
            self.set_battery_lvl()?; // podría llamarse recharge battery.

            // Vuelve a la posición correspondiente
            self.fly_to_mantainance(position_to_go, mqtt_client, true)?;
            self.data.set_state(state_to_set, true)?;
        }
        Ok(())
    }

    fn set_battery_lvl(&mut self) -> Result<(), Error> {
        if let Ok(mut ci) = self.current_info.lock() {
            ci.set_battery_lvl(self.dron_properties.get_max_battery_lvl());
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "Error al tomar lock de current info.",
            ))
        }
    }
}

#[cfg(test)]

mod test {
    use super::Dron;
    use crate::apps::sist_dron::calculations::calculate_direction;
    use crate::apps::sist_dron::dron_state::DronState;
    use crate::logging::string_logger::StringLogger;
    use std::sync::mpsc;

    fn create_dron_4() -> Dron {
        let (str_logger_tx, _str_logger_rx) = mpsc::channel::<String>();
        let logger = StringLogger::new(str_logger_tx); // para testing alcanza con crearlo así.

        // Dron 4 inicia en: -34.60282, -58.38730
        let lat = -34.60282;
        let lon = -58.38730;

        Dron::new_internal(4, lat, lon, logger).unwrap()
    }

    #[test]
    fn test_1_dron_se_inicia_con_id_y_estado_correctos() {
        let dron = create_dron_4();

        assert_eq!(dron.data.get_id().unwrap(), 4);
        assert_eq!(
            dron.data.get_state().unwrap(),
            DronState::ExpectingToRecvIncident
        ); // estado activo
    }

    #[test]
    fn test_2_dron_se_inicia_con_posicion_correcta() {
        let dron = create_dron_4();

        // El dron inicia desde esta posición.
        assert_eq!(
            dron.data.get_current_position().unwrap(),
            dron.dron_properties.get_range_center_position()
        );
    }

    #[test]
    fn test_3a_calculate_direction_da_la_direccion_esperada() {

        // Dados destino y origen
        let origin = (0.0, 0.0); // desde el (0,0)
        let destination = (4.0, -3.0);
        let hip = 5.0; // hipotenusa da 5;

        let dir = calculate_direction(origin, destination);

        // La dirección calculada es la esperada
        let expected_dir = (4.0 / hip, -3.0 / hip);
        assert_eq!(dir, expected_dir);
        // En "hip" cantidad de pasos, se llega a la posición de destino
        assert_eq!(origin.0 + dir.0 * hip, destination.0);
        assert_eq!(origin.1 + dir.1 * hip, destination.1);
    }

    #[test]
    fn test_3b_calculate_direction_da_la_direccion_esperada() {
        let dron = create_dron_4();

        // Dados destino y origen
        let origin = dron.data.get_current_position().unwrap(); // desde (incident_position, candidate_dron) que no es el (0,0)
        let destination = (origin.0 + 4.0, origin.1 - 3.0);
        let hip = 5.0; // hipotenusa da 5;

        let dir = calculate_direction(origin, destination);

        // La dirección calculada es la esperada
        let expected_dir = (4.0 / hip, -3.0 / hip);
        assert_eq!(dir, expected_dir);
        // En "hip" cantidad de pasos, se llega a la posición de destino
        assert_eq!(origin.0 + dir.0 * hip, destination.0);
        assert_eq!(origin.1 + dir.1 * hip, destination.1);
    }
}
