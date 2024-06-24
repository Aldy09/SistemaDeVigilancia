use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    net::SocketAddr,
    sync::{mpsc, Arc, Mutex},
    thread::{self, sleep, JoinHandle},
    time::Duration,
};

use std::sync::mpsc::{Receiver as MpscReceiver, Sender as MpscSender};

use crate::{
    apps::incident_state::IncidentState, mqtt::client::mqtt_client::MQTTClient,
    mqtt::messages::publish_message::PublishMessage,
};
use crate::{
    apps::{
        apps_mqtt_topics::AppsMqttTopics, common_clients::join_all_threads, incident::Incident,
        sist_dron::dron_state::DronState,
    },
    logging::{
        logger::Logger,
        structs_to_save_in_logger::{OperationType, StructsToSaveInLogger},
    },
    mqtt::messages::message_type::MessageType,
};

use super::{
    dron_current_info::DronCurrentInfo, dron_flying_info::DronFlyingInfo,
    sist_dron_properties::SistDronProperties,
};

type DistancesType = Arc<Mutex<HashMap<u8, ((f64, f64), Vec<(u8, f64)>)>>>; // (inc_id, ( (inc_pos),(dron_id, distance_to_incident)) )

/// Struct que representa a cada uno de los drones del sistema de vigilancia.
/// Al publicar en el topic `dron`, solamente el struct `DronCurrentInfo` es lo que interesa enviar,
/// ya que lo demás son constantes para el funcionamiento del Dron.
#[derive(Debug)]
pub struct Dron {
    // El id y su posición y estado actuales se encuentran en el siguiente struct
    current_info: Arc<Mutex<DronCurrentInfo>>,

    // Constantes cargadas desde un arch de configuración
    dron_properties: SistDronProperties,

    logger_tx: mpsc::Sender<StructsToSaveInLogger>,

    drone_distances_by_incident: DistancesType,
}

#[allow(dead_code)]
impl Dron {
    /// Dron se inicia con batería al 100%, desde la posición del range_center, con estado activo.
    pub fn new(id: u8, broker_addr: SocketAddr) -> Result<Self, Error> {
        let (logger_tx, logger_rx) = mpsc::channel::<StructsToSaveInLogger>();
        let mut dron = Self::new_internal(id, logger_tx)?;

        // Connect a server mqtt
        match dron.establish_mqtt_broker_connection(&broker_addr) {
            Ok(mut mqtt_client) => {
                // Publica su estado inicial
                if let Ok(ci) = &dron.current_info.lock() {
                    mqtt_client.mqtt_publish(AppsMqttTopics::DronTopic.to_str(), &ci.to_bytes())?;
                }

                dron.spawn_threads(mqtt_client, logger_rx)?;
            }
            Err(_) => {
                return Err(Error::new(
                    std::io::ErrorKind::Other,
                    "Error al establecer la conexion",
                ))
            }
        }

        Ok(dron)
    }

    pub fn spawn_threads(
        &mut self,
        mqtt_client: MQTTClient,
        logger_rx: MpscReceiver<StructsToSaveInLogger>,
    ) -> Result<(), Error> {
        let logger = Logger::new(logger_rx);
        let mut children: Vec<JoinHandle<()>> = vec![];
        let mqtt_client_sh = Arc::new(Mutex::new(mqtt_client));

        children.push(spawn_dron_stuff_to_logger_thread(logger));

        self.subscribe_to_topics(Arc::clone(&mqtt_client_sh))?;

        join_all_threads(children);

        Ok(())
    }

    pub fn clone_ref(&self) -> Self {
        Self {
            current_info: Arc::clone(&self.current_info),
            dron_properties: self.dron_properties,
            logger_tx: self.logger_tx.clone(),
            drone_distances_by_incident: Arc::clone(&self.drone_distances_by_incident),
        }
    }

    /// Se suscribe a topics inc y dron, y lanza la recepción de mensajes y finalización.
    fn subscribe_to_topics(&mut self, mqtt_client: Arc<Mutex<MQTTClient>>) -> Result<(), Error> {
        self.subscribe_to_topic(&mqtt_client, AppsMqttTopics::IncidentTopic.to_str());
        self.subscribe_to_topic(&mqtt_client, AppsMqttTopics::DronTopic.to_str());
        self.receive_messages_from_subscribed_topics(&mqtt_client);
        self.finalize_mqtt_client(&mqtt_client)?;
        Ok(())
    }

    /// Se suscribe al topic recibido.
    pub fn subscribe_to_topic(&self, mqtt_client: &Arc<Mutex<MQTTClient>>, topic: &str) {
        if let Ok(mut mqtt_client) = mqtt_client.lock() {
            let res_sub = mqtt_client.mqtt_subscribe(vec![(String::from(topic))]);
            match res_sub {
                Ok(subscribe_message) => {
                    self.logger_tx
                        .send(StructsToSaveInLogger::MessageType(
                            "Dron".to_string(),
                            MessageType::Subscribe(subscribe_message),
                            OperationType::Sent,
                        ))
                        .unwrap();
                }
                Err(e) => println!("Cliente: Error al hacer un subscribe a topic: {:?}", e),
            }
        }
    }

    /// Recibe mensajes de los topics a los que se ha suscrito: inc y dron.
    /// (aux sist monitoreo actualiza el estado del incidente y hace publish a inc; dron hace publish a dron)
    pub fn receive_messages_from_subscribed_topics(
        &mut self,
        mqtt_client: &Arc<Mutex<MQTTClient>>,
    ) {
        let mut children = vec![];
        loop {
            if let Ok(mqtt_client_l) = mqtt_client.lock() {
                match mqtt_client_l.mqtt_receive_msg_from_subs_topic() {
                    //Publish message: Incidente o dron
                    Ok(publish_message) => {
                        self.logger_tx
                            .send(StructsToSaveInLogger::MessageType(
                                "Dron".to_string(),
                                MessageType::Publish(publish_message.clone()),
                                OperationType::Received,
                            ))
                            .unwrap();
                        let handle_thread =
                            self.spawn_process_recvd_msg_thread(publish_message, mqtt_client);
                        children.push(handle_thread);
                    }
                    Err(e) => {
                        if !self.handle_message_receiving_error(e) {
                            break;
                        }
                    }
                }
            }
        }

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
        match msg.get_topic().as_str() {
            "Inc" => self.process_valid_inc(msg.get_payload(), mqtt_client),
            "Dron" => {
                let received_ci = DronCurrentInfo::from_bytes(msg.get_payload())?;
                let not_myself = self.get_id()? != received_ci.get_id();
                let recvd_dron_is_not_flying = received_ci.get_state() != DronState::Flying;
                let recvd_dron_is_not_managing_incident = received_ci.get_state() != DronState::ManagingIncident;

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
        let inc = Incident::from_bytes(payload);
        let event = format!("Recibo Inc: {:?}", inc);
        println!("{:?}", event);
        match *inc.get_state() {
            IncidentState::ActiveIncident => self.manage_incident(inc, mqtt_client),
            IncidentState::ResolvedIncident => {
                self.go_back_if_my_inc_was_resolved(inc, mqtt_client)?;
                Ok(())
            }
        }
    }

    /// Por cada dron recibido si tenemos un incidente en comun se actualiza el hashmap con la menor distancia al incidente entre los drones (self_distance y recibido_distance).
    fn process_valid_dron(&self, received_dron: DronCurrentInfo) -> Result<(), Error> {
        //Obtengo el ID del incidente que el dron recibido está atendiendo
        if let Some(inc_id) = received_dron.get_inc_id_to_resolve() {
            if let Ok(mut distances) = self.drone_distances_by_incident.lock() {
                println!("HOLA TOPIC DRON, entrando, el hashmap: {:?}", distances);
                //Si el incidente ya está en el hashmap, agrego la menor distancia al incidente entre los dos drones. Si no, lo ignoro porque la rama "topic inc" no lo marco como de interes.
                if let Some((incident_position, candidate_drones)) = distances.get_mut(&inc_id) {
                    let received_dron_distance = received_dron.get_distance_to(*incident_position);

                    let self_distance = self.get_distance_to(*incident_position)?;

                    println!("HOLA TOPIC DRON, antes de pushear self_distance: {:?}, self_distance: {:?}", self_distance, received_dron_distance);
                    //Agrego al vector la menor distancia entre los dos drones al incidente
                    if self_distance <= received_dron_distance {
                        candidate_drones.push((self.get_id()?, self_distance));
                    } else {
                        candidate_drones.push((received_dron.get_id(), received_dron_distance));
                    }

                    println!(
                        "HOLA TOPIC DRON, he pusheado el de menor dist, el hashmap: {:?}",
                        distances
                    );
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
        println!("HOLA Esperando para recibir notificaciones de otros drones...");
        //thread::sleep(Duration::from_millis(500));
        thread::sleep(Duration::from_millis(3500)); // Aux Probando
        if let Ok(mut distances) = self.drone_distances_by_incident.lock() {
            println!(
                "HOLA adentro del decide, estoy por consultarlo, el hashmap: {:?}",
                distances
            );
            if let Some((_incident_position, candidate_drones)) =
                distances.get_mut(&incident.get_id())
            {
                // Ordenar por el valor f64 de la tupla, de menor a mayor
                candidate_drones.sort_by(|a, b| a.1.total_cmp(&b.1));

                // Seleccionar los primeros dos elementos después de ordenar
                let closest_two_drones: Vec<u8> =
                    candidate_drones.iter().take(2).map(|&(id, _)| id).collect();

                // Si el id del dron actual está en la lista de los dos más cercanos, entonces se mueve
                should_move = closest_two_drones.contains(&self.get_id()?);

                println!("HOLA después del contains, el bool da: {}", should_move);

                // Si está vacío, no se recibió aviso de un dron más cercano, entonces voy yo
                if closest_two_drones.is_empty() || closest_two_drones.len() == 1 {
                    should_move = true;
                }
                println!("HOLA después del is_empty, el bool da: {}", should_move);
            } else {
                println!("HOLA ESTO NUNCA DEBERÍA PASAR: {}", should_move);
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
        let event = format!("Recibido inc activo de id: {}", inc_id.get_id());
        println!("{:?}", event);

        // Analizar condiciones para saber si se desplazará a la pos del incidente
        //  - batería es mayor al nivel bateria minima
        let batery_lvl = self.get_battery_lvl()?;
        let enough_battery = batery_lvl >= self.dron_properties.get_min_operational_battery_lvl();
        //  - inc.pos dentro del rango
        let (inc_lat, inc_lon) = inc_id.get_position();
        let inc_in_range =
            self.is_within_range_from_self(inc_lat, inc_lon, self.dron_properties.get_range());

        if enough_battery {
            if inc_in_range {
                println!("Dio true, evaluaré si tengo una de las dos menores distancias al inc.");
                self.set_inc_id_to_resolve(inc_id.get_id())?; // Aux: ver si va acá o con la "condición b". [].
                self.add_incident_to_hashmap(&inc_id)?;

                self.set_state(DronState::RespondingToIncident)?;

                // Hace publish de su estado (de su current info) _ le servirá a otros drones para ver la condición b, y monitoreo para mostrarlo en mapa
                if let Ok(mut mqtt_client_l) = mqtt_client.lock() {
                    if let Ok(ci) = &self.current_info.lock() {
                        mqtt_client_l
                            .mqtt_publish(AppsMqttTopics::DronTopic.to_str(), &ci.to_bytes())?;
                    }
                };

                let should_move =
                    self.decide_if_should_move_to_incident(&inc_id, mqtt_client.clone())?;
                println!("Dio que debería moverme: {}", should_move);
                if should_move {
                    // Volar hasta la posición del incidente
                    let destination = inc_id.get_position();
                    self.fly_to(destination, mqtt_client)?;
                }
            } else {
                println!("print aux: el inc No está en mi rango.")
            }
        } else {
            println!("Sin suficiente batería para resolver el inc, vuelo a mantenimiento.");
            // No tiene suficiente batería, por lo que debe ir a mantenimiento a recargarse
            self.set_state(DronState::Mantainance)?;

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
        println!(
            "Dio que la cuenta vale: {}, y adj_range vale: {}. Era rango: {}",
            rad, adjusted_range, range
        ); // debug []

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
        let event = format!("Recibido inc resuelto de id: {}", inc.get_id());
        println!("{:?}", event);

        if let Some(my_inc_id) = self.get_inc_id_to_resolve()? {
            if inc.get_id() == my_inc_id {
                self.go_back_to_range_center_position(mqtt_client)?;
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
        self.set_state(DronState::ExpectingToRecvIncident)?;

        Ok(())
    }

    /// Calcula la dirección en la que debe volar desde una posición `origin` hasta `destination`.
    // Aux: esto estaría mejor en un struct posicion quizás? [] ver.
    fn calculate_direction(&self, origin: (f64, f64), destination: (f64, f64)) -> (f64, f64) {
        // calcular la distancia ('en diagonal') entre los dos puntos
        let (origin_lat, origin_lon) = (origin.0, origin.1);
        let (dest_lat, dest_lon) = (destination.0, destination.1);

        // Cálculo de distancia
        let lat_dist = dest_lat - origin_lat;
        let lon_dist = dest_lon - origin_lon;
        let distance = f64::sqrt(lat_dist.powi(2) + lon_dist.powi(2));

        // Vector unitario: (destino - origen) / || distancia ||, para cada coordenada.
        let unit_lat = lat_dist / distance;
        let unit_lon = lon_dist / distance;
        let direction: (f64, f64) = (unit_lat, unit_lon);

        direction
    }

    fn calculate_distance(&self, a: (f64, f64), b: (f64, f64)) -> f64 {
        ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
    }

    fn fly_to(
        &mut self,
        destination: (f64, f64),
        mqtt_client: &Arc<Mutex<MQTTClient>>,
    ) -> Result<(), Error> {
        let origin = self.get_current_position()?;
        let dir = self.calculate_direction(origin, destination);
        println!(
            "Fly_to: dir: {:?}, vel: {}",
            dir,
            self.dron_properties.get_speed()
        );
        self.set_state(DronState::Flying)?;
        self.set_flying_info_values(dir)?;
    
        let mut current_pos = origin;
        let threshold = 0.0001; // Define un umbral adecuado para tu aplicación
        while self.calculate_distance(current_pos, destination) > threshold {
            current_pos = self.increment_current_position_in(dir)?;
    
            // Simular el vuelo, el dron se desplaza
            let a = 300; // aux
            sleep(Duration::from_micros(a));
    
            println!(
                "Dron: incrementé mi posición, pos actual: {:?}",
                self.get_current_position()
            );
            // Hace publish de su estado (de su current info)
            // Hace publish de su estado (de su current info)
            if let Ok(mut mqtt_client_l) = mqtt_client.lock() {
                if let Ok(ci) = &self.current_info.lock() {
                    mqtt_client_l
                        .mqtt_publish(AppsMqttTopics::DronTopic.to_str(), &ci.to_bytes())?;
                }
            };
        }
    
        // Al llegar, el dron ya no se encuentra en desplazamiento.
        self.unset_flying_info_values()?;
        println!(
            "Dron: llegué a destino [todavía aprox], pos actual: {:?}",
            self.get_current_position()
        );

        // Llegue a destino entonces debo cambiar a estado --> Manejando Incidente
        self.set_state(DronState::ManagingIncident)?;

        // Hace publish de su estado (de su current info)
        if let Ok(mut mqtt_client_l) = mqtt_client.lock() {
            if let Ok(ci) = &self.current_info.lock() {
                mqtt_client_l.mqtt_publish(AppsMqttTopics::DronTopic.to_str(), &ci.to_bytes())?;
            }
        };
    
        println!("Fin vuelo hasta incidente.");

        Ok(())
    } 

    /// Establece como `flying_info` a la dirección recibida, y a la velocidad leída del archivo de configuración.
    fn set_flying_info_values(&mut self, dir: (f64, f64)) -> Result<(), Error> {
        let speed = self.dron_properties.get_speed();
        let info = DronFlyingInfo::new(dir, speed);
        self.set_flying_info(info)?;
        Ok(())
    }

    /// Establece `None` como `flying_info`, lo cual indica que el dron no está actualmente en desplazamiento.
    /// Toma lock en el proceso.
    fn unset_flying_info_values(&mut self) -> Result<(), Error> {
        if let Ok(mut ci) = self.current_info.lock() {
            ci.unset_flying_info();
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    //// Funciones que toman lock ////

    /// Toma lock y devuelve su nivel de batería.
    fn get_battery_lvl(&self) -> Result<u8, Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.get_battery_lvl());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    /// Toma lock y establece el inc id a resolver.
    fn set_inc_id_to_resolve(&self, inc_id: u8) -> Result<(), Error> {
        if let Ok(mut ci) = self.current_info.lock() {
            ci.set_inc_id_to_resolve(inc_id);
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    fn set_state(&self, new_state: DronState) -> Result<(), Error> {
        if let Ok(mut ci) = self.current_info.lock() {
            ci.set_state(new_state);
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    fn get_current_position(&self) -> Result<(f64, f64), Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.get_current_position());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    fn increment_current_position_in(&self, dir: (f64, f64)) -> Result<(f64, f64), Error> {
        if let Ok(mut ci) = self.current_info.lock() {
            return Ok(ci.increment_current_position_in(dir));
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

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

    fn get_inc_id_to_resolve(&self) -> Result<Option<u8>, Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.get_inc_id_to_resolve());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    fn get_id(&self) -> Result<u8, Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.get_id());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    fn get_state(&self) -> Result<DronState, Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.get_state());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    fn get_distance_to(&self, destination: (f64, f64)) -> Result<f64, Error> {
        if let Ok(ci) = self.current_info.lock() {
            return Ok(ci.get_distance_to(destination));
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de current info.",
        ))
    }

    /// Dron se inicia con batería al 100%, desde la posición del range_center, con estado activo.
    /// Función utilizada para testear, no necesita broker address.
    fn new_internal(id: u8, logger_tx: MpscSender<StructsToSaveInLogger>) -> Result<Self, Error> {
        // Se cargan las constantes desde archivo de config.
        let properties_file = "src/apps/sist_dron/sistema_dron.properties";
        let dron_properties = SistDronProperties::new(properties_file)?;
        let drone_distances_by_incident = Arc::new(Mutex::new(HashMap::new()));

        // Inicia desde el range_center, por lo cual tiene estado 1 (activo); y con batería al 100%.
        // Aux, #ToDo, hacer una función para que la posición rance_center sea distinta para cada dron
        // aux: ej que tomen la get_range_center_position como base, y se ubiquen (ej en grilla) con + self id*factor (o (incident_position, candidate_dron) por el estilo).
        let (rng_center_lat, rng_center_lon) = dron_properties.get_range_center_position();
        //Posicion inicial del dron
        let (lat_inicial, lon_inicial) =
            calculate_initial_position(rng_center_lat, rng_center_lon, id);
        let current_info = DronCurrentInfo::new(
            id,
            /*
            rng_center_lat,
            rng_center_lon,
            */
            lat_inicial,
            lon_inicial,
            100,
            DronState::ExpectingToRecvIncident,
        );

        println!(
            "Dron {} se crea en posición (lat, lon): {}, {}.",
            id, rng_center_lat, rng_center_lon
        );

        let dron = Dron {
            current_info: Arc::new(Mutex::new(current_info)),
            // Las siguientes son las constantes, que vienen del arch de config:
            dron_properties,
            logger_tx,
            drone_distances_by_incident,
            /*max_battery_lvl: 100,
            min_operational_battery_lvl: 20,
            range: 40,
            stay_at_inc_time: 200,
            range_center_lat: range_center_lat_property,
            range_center_lon: range_center_lon_property,
            mantainance_lat: -34.30,
            mantainance_lon: -58.30,*/
        };

        Ok(dron)
    }

    /// Crea el client_id a partir de sus datos. Obtiene la broker_addr del server a la que conectarse, a partir de
    /// los argumentos ingresados al llamar al main. Y llama al connect de mqtt.
    pub fn establish_mqtt_broker_connection(
        &self,
        broker_addr: &SocketAddr,
    ) -> Result<MQTTClient, Error> {
        if let Ok(ci) = self.current_info.lock() {
            let client_id = format!("dron-{}", ci.get_id());
            let mqtt_client = MQTTClient::mqtt_connect_to_broker(client_id.as_str(), broker_addr)?;
            println!("Cliente: Conectado al broker MQTT.");

            return Ok(mqtt_client);
        };

        Err(Error::new(
            ErrorKind::ConnectionRefused,
            "Error al conectarse a mqtt server",
        ))
    }

    /// Finaliza la conexión con server.
    fn finalize_mqtt_client(&self, mqtt_client: &Arc<Mutex<MQTTClient>>) -> Result<(), Error> {
        if let Ok(mut mqtt_client) = mqtt_client.lock().map_err(|_| {
            Error::new(
                ErrorKind::Other,
                "Error al intentar tomar lock para suscribirse.",
            )
        }) {
            mqtt_client.finish();
        }
        Ok(())
    }

    // Aux: puede estar en un common xq es copypaste de la de monitoreo
    fn handle_message_receiving_error(&self, e: std::io::Error) -> bool {
        match e.kind() {
            std::io::ErrorKind::TimedOut => true,
            std::io::ErrorKind::NotConnected => {
                println!("Cliente: No hay más PublishMessage's por leer.");
                false
            }
            _ => {
                println!("Cliente: error al leer los publish messages recibidos.");
                true
            }
        }
    }

    fn add_incident_to_hashmap(&self, inc_id: &Incident) -> Result<(), Error> {
        if let Ok(mut distances) = self.drone_distances_by_incident.lock() {
            println!("HOLA antes del add to hashmap, el hashmap: {:?}", distances);
            distances.insert(inc_id.get_id(), (inc_id.get_position(), Vec::new()));
            println!("HOLA dsp del add to hashmap, el hashmap: {:?}", distances);
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::Other,
            "Error al tomar lock de drone_distances_by_incident.",
        ))
    }
}

fn spawn_dron_stuff_to_logger_thread(logger: Logger) -> JoinHandle<()> {
    thread::spawn(move || loop {
        while let Ok(msg) = logger.logger_rx.recv() {
            logger.write_in_file(msg);
        }
    })
}

/// Calcula la posición inicial del dron, basada en el id del dron.
/// Funciona para cualquier número de drones.
/// Una distancia de aproximadamente 4 cuadras entre cada dron.
pub fn calculate_initial_position(rng_center_lat: f64, rng_center_lon: f64, id: u8) -> (f64, f64) {
    // Asume que cada fila puede tener hasta 3 drones
    let drones_por_fila = 3;

    // Calcula la fila y la columna basándose en el id, asumiendo que id comienza en 1
    let row = (id - 1) / drones_por_fila;
    let col = (id - 1) % drones_por_fila;

    // Calcula la nueva latitud y longitud basada en la fila y columna
    let lat = rng_center_lat + row as f64 * 0.00618; // Ajusta estos valores según la distancia deseada
    let lon = rng_center_lon + col as f64 * 0.00618;

    (lat, lon)
}

#[cfg(test)]

mod test {
    use std::sync::mpsc;

    use crate::apps::sist_dron::dron_state::DronState;
    use crate::logging::structs_to_save_in_logger::StructsToSaveInLogger;

    use super::Dron;

    use super::calculate_initial_position;

    #[test]
    fn test_1_dron_se_inicia_con_id_y_estado_correctos() {
        let (logger_tx, _logger_rx) = mpsc::channel::<StructsToSaveInLogger>();

        let dron = Dron::new_internal(1, logger_tx).unwrap();

        assert_eq!(dron.get_id().unwrap(), 1);
        assert_eq!(
            dron.get_state().unwrap(),
            DronState::ExpectingToRecvIncident
        ); // estado activo
    }

    #[test]
    fn test_2_dron_se_inicia_con_posicion_correcta() {
        let (logger_tx, _logger_rx) = mpsc::channel::<StructsToSaveInLogger>();

        let dron = Dron::new_internal(1, logger_tx).unwrap();

        // El dron inicia desde esta posición.
        // Aux, #ToDo: para que inicien desde su range center real, y no todos desde el mismo punto del mapa,
        //  aux: quizás sería necesario involucrar al id en la cuenta, ej una lat base + id*algún_factor, para espaciarlos en el mapa al iniciar. Ver [].
        assert_eq!(
            dron.get_current_position().unwrap(),
            dron.dron_properties.get_range_center_position()
        );
    }

    #[test]
    fn test_3a_calculate_direction_da_la_direccion_esperada() {
        let (logger_tx, _logger_rx) = mpsc::channel::<StructsToSaveInLogger>();

        let dron = Dron::new_internal(1, logger_tx).unwrap();

        // Dados destino y origen
        let origin = (0.0, 0.0); // desde el (0,0)
        let destination = (4.0, -3.0);
        let hip = 5.0; // hipotenusa da 5;

        let dir = dron.calculate_direction(origin, destination);

        // La dirección calculada es la esperada
        let expected_dir = (4.0 / hip, -3.0 / hip);
        assert_eq!(dir, expected_dir);
        // En "hip" cantidad de pasos, se llega a la posición de destino
        assert_eq!(origin.0 + dir.0 * hip, destination.0);
        assert_eq!(origin.1 + dir.1 * hip, destination.1);
    }

    #[test]
    fn test_3b_calculate_direction_da_la_direccion_esperada() {
        let (logger_tx, _logger_rx) = mpsc::channel::<StructsToSaveInLogger>();

        let dron = Dron::new_internal(1, logger_tx).unwrap();

        // Dados destino y origen
        let origin = dron.get_current_position().unwrap(); // desde (incident_position, candidate_dron) que no es el (0,0)
        let destination = (origin.0 + 4.0, origin.1 - 3.0);
        let hip = 5.0; // hipotenusa da 5;

        let dir = dron.calculate_direction(origin, destination);

        // La dirección calculada es la esperada
        let expected_dir = (4.0 / hip, -3.0 / hip);
        assert_eq!(dir, expected_dir);
        // En "hip" cantidad de pasos, se llega a la posición de destino
        assert_eq!(origin.0 + dir.0 * hip, destination.0);
        assert_eq!(origin.1 + dir.1 * hip, destination.1);
    }

    #[test]
    fn test_4_calcula_correctamente_posiciones_inciales() {
        let rng_center_lat = 10.0;
        let rng_center_lon = 20.0;

        // Test para el primer dron
        let id1 = 1;
        let expected_position1 = (10.0, 20.0); // Asume que el primer dron inicia en el centro de rango
        let position1 = calculate_initial_position(rng_center_lat, rng_center_lon, id1);
        assert_eq!(position1, expected_position1);

        // Test para un dron en la segunda columna de la primera fila
        let id3 = 2;
        let expected_position3 = (10.0, 20.00618); // Asume ajuste de columna sin cambio en fila
        let position3 = calculate_initial_position(rng_center_lat, rng_center_lon, id3);
        assert_eq!(position3, expected_position3);

        // Test para un dron en la segunda fila
        let id2 = 4;
        let expected_position2 = (10.00618, 20.0); // Asume ajuste de fila sin cambio en columna
        let position2 = calculate_initial_position(rng_center_lat, rng_center_lon, id2);
        assert_eq!(position2, expected_position2);
    }

    #[test]
    fn test_4a_drones_1_2_3_same_latitude() {
        let rng_center_lat = 10.0;
        let rng_center_lon = 20.0;

        // Calcula las posiciones para los drones 1, 2 y 3
        let position1 = calculate_initial_position(rng_center_lat, rng_center_lon, 1);
        let position2 = calculate_initial_position(rng_center_lat, rng_center_lon, 2);
        let position3 = calculate_initial_position(rng_center_lat, rng_center_lon, 3);

        // Verifica que los drones 1, 2 y 3 estén en la misma latitud
        assert_eq!(position1.0, position2.0);
        assert_eq!(position2.0, position3.0);
    }

    #[test]
    fn test_4b_drones_1_4_7_same_longitude() {
        let rng_center_lat = 10.0;
        let rng_center_lon = 20.0;

        // Calcula las posiciones para los drones 1, 4 y 7
        let position1 = calculate_initial_position(rng_center_lat, rng_center_lon, 1);
        let position4 = calculate_initial_position(rng_center_lat, rng_center_lon, 4);
        let position7 = calculate_initial_position(rng_center_lat, rng_center_lon, 7);

        // Verifica que los drones 1, 4 y 7 estén en la misma longitud
        assert_eq!(position1.1, position4.1);
        assert_eq!(position4.1, position7.1);
    }

    #[test]
    fn test_drones_8_9_10_same_longitude_distance() {
        let rng_center_lat = 10.0;
        let rng_center_lon = 20.0;

        // Calcula las posiciones para los drones 8, 9 y 10
        let position8 = calculate_initial_position(rng_center_lat, rng_center_lon, 7);
        let position9 = calculate_initial_position(rng_center_lat, rng_center_lon, 8);
        let position10 = calculate_initial_position(rng_center_lat, rng_center_lon, 9);

        // Extrae las longitudes
        let lon8 = position8.1;
        let lon9 = position9.1;
        let lon10 = position10.1;

        // Calcula las diferencias de longitud
        let diff_8_9 = (lon9 - lon8).abs();
        let diff_9_10 = (lon10 - lon9).abs();

        // Verifica que las diferencias de longitud sean iguales
        assert_eq!(diff_8_9, diff_9_10);
    }
}
