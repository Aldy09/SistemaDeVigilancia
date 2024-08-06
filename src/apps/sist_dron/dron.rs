use std::{
    collections::HashMap,
    fs,
    io::{self, Error, ErrorKind},
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

use std::sync::mpsc::Receiver as MpscReceiver;

use crate::apps::{common_clients::there_are_no_more_publish_msgs,
    incident_data::incident_info::IncidentInfo,
};
use crate::apps::{
    apps_mqtt_topics::AppsMqttTopics, common_clients::join_all_threads,
    sist_dron::dron_state::DronState,
};
use crate::logging::string_logger::StringLogger;
use crate::mqtt::{client::mqtt_client::MQTTClient, messages::publish_message::PublishMessage};

use super::{
    battery_manager::BatteryManager, data::Data, dron_current_info::DronCurrentInfo, dron_logic::DronLogic, sist_dron_properties::SistDronProperties
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
        
        let (ci_tx, ci_rx) = mpsc::channel::<DronCurrentInfo>();
        children.push(self.spawn_for_update_battery(ci_tx.clone()));
        
        children.push(self.spawn_recv_ci_and_publish(ci_rx, mqtt_client_sh.clone()));
        self.subscribe_to_topics(mqtt_client_sh.clone(), mqtt_rx, ci_tx)?;
        
        Ok(children)
    }
    
    fn spawn_for_update_battery(&self,ci_tx: mpsc::Sender<DronCurrentInfo>) -> JoinHandle<()> {
        let s = self.clone_ref();
        thread::spawn(move || {
            let mut battery_manager = BatteryManager::new(s.data,
                s.dron_properties, s.logger, ci_tx);
            battery_manager.run();
        })
    }

    pub fn clone_ref(&self) -> Self {
        Self {
            data: self.data.clone_ref(),
            current_info: Arc::clone(&self.current_info),
            dron_properties: self.dron_properties,
            logger: self.logger.clone_ref(),
            drone_distances_by_incident: Arc::clone(&self.drone_distances_by_incident),
            qos: self.qos,
        }
    }

    /// Recibe por rx la current_info que se desea publicar, y la publica por MQTT.
    pub fn spawn_recv_ci_and_publish(&self, ci_rx: mpsc::Receiver<DronCurrentInfo>, mqtt_client: Arc<Mutex<MQTTClient>>) -> JoinHandle<()>  {
        let self_c = self.clone_ref();
        thread::spawn(move || {
            for ci in ci_rx {
                if let Err(e) = self_c.publish_current_info(ci, &mqtt_client) {
                        self_c.logger.log(format!("Error al publicar la current_info: {:?}.", e));
                }
            }                        
        })
    }

    /// Hace publish de su current info.
    /// Le servirá a otros drones para ver la condición de los dos drones más cercanos y a monitoreo para mostrarlo en mapa.
    pub fn publish_current_info(&self, _ci: DronCurrentInfo, mqtt_client: &Arc<Mutex<MQTTClient>>) -> Result<(), Error> {
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

    /// Se suscribe a topics inc y dron, y lanza la recepción de mensajes y finalización.
    fn subscribe_to_topics(
        &mut self,
        mqtt_client: Arc<Mutex<MQTTClient>>,
        mqtt_rx: MpscReceiver<PublishMessage>,
        ci_tx: mpsc::Sender<DronCurrentInfo>,
    ) -> Result<(), Error> {
        self.subscribe_to_topic(&mqtt_client, AppsMqttTopics::IncidentTopic.to_str())?;
        self.subscribe_to_topic(&mqtt_client, AppsMqttTopics::DronTopic.to_str())?;
        self.receive_messages_from_subscribed_topics(&mqtt_client, mqtt_rx, ci_tx);

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
        ci_tx: mpsc::Sender<DronCurrentInfo>
    ) {
        let mut children = vec![];

        let self_child = self.clone_ref();
        let self_child_aux = self.clone_ref();
        let dron_logic = DronLogic::new(self_child.data,
            self_child.dron_properties, mqtt_client.clone(), self_child.logger,
             self_child_aux, self_child.drone_distances_by_incident.clone(), ci_tx);
            // Aux: el mqtt_client y el dron se los paso solo por ahora, dsp los vamos a borrar de ahí.

        for publish_msg in mqtt_rx {
            self.logger
                .log(format!("Dron: Recibo mensaje Publish: {:?}", publish_msg));

            // Lanza un hilo para procesar el mensaje, y luego lo espera correctamente
            let handle_thread = self.spawn_process_recvd_msg_thread(publish_msg, dron_logic.clone_ref());
            children.push(handle_thread);
        }
        there_are_no_more_publish_msgs(&self.logger);

        join_all_threads(children);
    }

    fn spawn_process_recvd_msg_thread(
        &self,
        msg: PublishMessage,
        dron_logic: DronLogic,
    ) -> JoinHandle<()> {
        let mut logic_clone = dron_logic.clone_ref();
        let logger_c = self.logger.clone_ref();
        thread::spawn(move || {
            if let Err(e) = logic_clone.process_recvd_msg(msg) {
                logger_c.log(format!("Error al procesar mensage recibido, process_rcvd_msg: {:?}.", e));
            }
        })
    }

    // Aux: esta función no va a estar más acá dsp de un refactor. [].
    pub fn get_distance_to(&self, destination: (f64, f64)) -> Result<f64, Error> {
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
        let data = Data::new(current_info.clone());
        let dron = Dron {
            data,
            current_info,
            dron_properties,
            logger,
            drone_distances_by_incident,
            qos,
        };

        Ok(dron)
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
