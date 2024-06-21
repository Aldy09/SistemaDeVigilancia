use std::{
    io::{Error, ErrorKind}, net::SocketAddr, sync::{Arc, Mutex}, thread::sleep, time::Duration
};

use crate::apps::{apps_mqtt_topics::AppsMqttTopics, sist_dron::dron_state::DronState};
use crate::{
    apps::{incident::Incident, incident_state::IncidentState},
    mqtt::client::mqtt_client::MQTTClient,
    mqtt::messages::publish_message::PublishMessage,
};

use super::{
    dron_current_info::DronCurrentInfo, dron_flying_info::DronFlyingInfo,
    sist_dron_properties::SistDronProperties,
};

/// Struct que representa a cada uno de los drones del sistema de vigilancia.
/// Al publicar en el topic `dron`, solamente el struct `DronCurrentInfo` es lo que interesa enviar,
/// ya que lo demás son constantes para el funcionamiento del Dron.
#[derive(Debug)]
pub struct Dron {
    // El id y su posición y estado actuales se encuentran en el siguiente struct
    current_info: DronCurrentInfo,

    // Constantes cargadas desde un arch de configuración
    dron_properties: SistDronProperties,
}

#[allow(dead_code)]
impl Dron {
    /// Dron se inicia con batería al 100%, desde la posición del range_center, con estado activo.
    pub fn new(id: u8, broker_addr: SocketAddr) -> Result<Self, Error> {
                
        let mut dron = Self::new_internal(id)?;
        
        dron.run(&broker_addr)?;
        
        Ok(dron)
    }
    
    /// Dron se inicia con batería al 100%, desde la posición del range_center, con estado activo.
    /// Función utilizada para testear, no necesita broker address.
    fn new_internal(id: u8) -> Result<Self, Error> {
        // Se cargan las constantes desde archivo de config.
        let properties_file = "src/apps/sist_dron/sistema_dron.properties";
        let dron_properties = SistDronProperties::new(properties_file)?;

        // Inicia desde el range_center, por lo cual tiene estado 1 (activo); y con batería al 100%.
        // Aux, #ToDo, hacer una función para que la posición rance_center sea distinta para cada dron
        // aux: ej que tomen la get_range_center_position como base, y se ubiquen (ej en grilla) con + self id*factor (o algo por el estilo).
        let (rng_center_lat, rng_center_lon) = dron_properties.get_range_center_position();
        let current_info = DronCurrentInfo::new(
            id,
            rng_center_lat,
            rng_center_lon,
            100,
            DronState::ExpectingToRecvIncident,
        );
        
        println!("Dron {} se crea en posición (lat, lon): {}, {}.", id, rng_center_lat, rng_center_lon);

        let dron = Dron {
            current_info,
            // Las siguientes son las constantes, que vienen del arch de config:
            dron_properties,
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

    /// Ejecuta la aplicación de dron, para ponerlo en funcionamiento.
    pub fn run(&mut self, broker_addr: &SocketAddr) -> Result<(), Error>{
        // Connect a server mqtt
        let mqtt = self.establish_mqtt_broker_connection(broker_addr)?;
        let mqtt_client = Arc::new(Mutex::new(mqtt));
        
        // Publica su estado inicial
        if let Ok(mut mqtt_client_l) = mqtt_client.lock() {
            mqtt_client_l.mqtt_publish(AppsMqttTopics::DronTopic.to_str(), &self.current_info.to_bytes())?;
        };

        // Se suscribe y permanece escuchando por Messages recibidos
        self.subscribe_to_topics(mqtt_client)?;

        Ok(())
    }

    /// Crea el client_id a partir de sus datos. Obtiene la broker_addr del server a la que conectarse, a partir de
    /// los argumentos ingresados al llamar al main. Y llama al connect de mqtt.
    pub fn establish_mqtt_broker_connection(
        &self,
        broker_addr: &SocketAddr
    ) -> Result<MQTTClient, Error> {
        let client_id = format!("dron-{}", self.current_info.get_id());
        let mqtt_client = MQTTClient::mqtt_connect_to_broker(client_id.as_str(), broker_addr)?;
        println!("Cliente: Conectado al broker MQTT.");
        
        Ok(mqtt_client)       
    }

    /// Se suscribe a topics inc y dron, y lanza la recepción de mensajes y finalización.
    fn subscribe_to_topics(&mut self, mqtt_client: Arc<Mutex<MQTTClient>>) -> Result<(), Error> {        
        self.subscribe_to_topic(&mqtt_client, AppsMqttTopics::IncidentTopic.to_str())?;
        self.subscribe_to_topic(&mqtt_client, AppsMqttTopics::DronTopic.to_str())?;
        self.receive_messages_from_subscribed_topics(&mqtt_client);
        self.finalize_mqtt_client(&mqtt_client)?;
        Ok(())
    }
    
    /// Se suscribe al topic recibido.
    pub fn subscribe_to_topic(&self, mqtt_client: &Arc<Mutex<MQTTClient>>, topic: &str) -> Result<(), Error>{
        if let Ok(mut mqtt_client) = mqtt_client.lock()
            .map_err(|_| Error::new(ErrorKind::Other, "Error al intentar tomar lock para suscribirse.")) {
            
            mqtt_client.mqtt_subscribe(vec![(String::from(topic))])
                .map_err(|_| Error::new(ErrorKind::Other, "Error al hacer un subscribe a topic"))?; // aux: le habría pasado la "e" si hubiera sido un struct []
            
            println!("Cliente: Hecho un subscribe a topic {}", topic);            
        }
        Ok(())
    }

    /// Finaliza la conexión con server.
    fn finalize_mqtt_client(&self, mqtt_client: &Arc<Mutex<MQTTClient>>) -> Result<(), Error>{
        if let Ok(mut mqtt_client) = mqtt_client.lock()
            .map_err(|_| Error::new(ErrorKind::Other, "Error al intentar tomar lock para suscribirse.")) {
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

    /// Recibe mensajes de los topics a los que se ha suscrito: inc y dron.
    /// (aux sist monitoreo actualiza el estado del incidente y hace publish a inc; dron hace publish a dron)
    fn receive_messages_from_subscribed_topics(&mut self, mqtt_client: &Arc<Mutex<MQTTClient>>) {
        // Loop que lee msjs que le envía el mqtt_client
        loop {
            if let Ok(mqtt_client_l) = mqtt_client.lock() {
                match mqtt_client_l.mqtt_receive_msg_from_subs_topic() {
                    //Publish message: inc o dron
                    Ok(msg) => {
                        // aux, ver []: no quiero devolverlo, si lo devuelvo corto el loop, y yo quiero seguir leyendo
                        let _res = self.process_recvd_msg(msg, mqtt_client);
                    }
                    Err(e) => {
                        // Si es false, corta el loop porque no hay más mensajes por leer
                        if !self.handle_message_receiving_error(e) {
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Recibe un mensaje de los topics a los que se suscribió, y lo procesa.
    fn process_recvd_msg(
        &mut self,
        msg: PublishMessage,
        mqtt_client: &Arc<Mutex<MQTTClient>>,
    ) -> Result<(), Error> {
        match msg.get_topic().as_str() {
            "Inc" => self.process_valid_inc(msg.get_payload(), mqtt_client),
            "Dron" => self.process_valid_dron(msg.get_payload()),
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

    /// Publica su estado, y analiza condiciones para desplazarse.
    fn manage_incident(
        &mut self,
        incident: Incident,
        mqtt_client: &Arc<Mutex<MQTTClient>>,
    ) -> Result<(), Error> {
        let event = format!("Recibido inc activo de id: {}", incident.get_id());
        println!("{:?}", event);

        // Analizar condiciones para saber si se desplazará a la pos del incidente
        //  - batería es mayor al nivel bateria minima
        let enough_battery = self.current_info.get_battery_lvl()
            >= self.dron_properties.get_min_operational_battery_lvl();
        //  - inc.pos dentro del rango
        let (inc_lat, inc_lon) = incident.get_position();
        let inc_in_range =
            self.is_within_range_from_self(inc_lat, inc_lon, self.dron_properties.get_range());

        if enough_battery {
            if inc_in_range {
                println!("Dio true, me desplazaré a la pos del inc.");
                self.current_info.set_inc_id_to_resolve(incident.get_id()); // Aux: ver si va acá o con la "condición b". [].

                self.current_info.set_state(DronState::RespondingToIncident);

                // Volar hasta la posición del incidente
                let destination = incident.get_position();
                self.fly_to(destination, mqtt_client)?;
            } else {
                println!("print aux: el inc No está en mi rango.")
            }
        } else {
            println!("Sin suficiente batería para resolver el inc, vuelo a mantenimiento.");
            // No tiene suficiente batería, por lo que debe ir a mantenimiento a recargarse
            self.current_info.set_state(DronState::Mantainance);

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
        let adjusted_range = range / 10000.0; // hay que modificar el range de las cámaras, ahora que son latitudes de verdad y no "3 4".
        println!("Dio que la cuenta vale: {}, y adj_range vale: {}. Era rango: {}", rad, adjusted_range, range); // debug []

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

        if let Some(my_inc_id) = self.current_info.get_inc_id_to_resolve() {
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
        self.current_info
            .set_state(DronState::ExpectingToRecvIncident);

        Ok(())
    }

    // Aux: #ToDo
    fn process_valid_dron(&self, payload: Vec<u8>) -> Result<(), Error> {
        let dron = DronCurrentInfo::from_bytes(payload);
        let event = format!("Recibo Dron: {:?}", dron);
        println!("{:?}", event);
        //todo!();
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

    /// Vuela hasta la posición de destino.
    /// Desde que inicia el desplazamiento y hasta que llega posee flying_info
    /// (dirección y velocidad de desplazamiento). Al llegar a destino ya no posee esos valores.
    fn fly_to(
        &mut self,
        destination: (f64, f64),
        mqtt_client: &Arc<Mutex<MQTTClient>>,
    ) -> Result<(), Error> {
        let origin = self.current_info.get_current_position();
        let dir = self.calculate_direction(origin, destination);
        self.set_flying_info_values(dir);

        let mut current_pos = origin;
        while current_pos != destination {
            current_pos = self.current_info.increment_current_position_in(dir);

            // Simular el vuelo, el dron se desplaza
            let a = 1; // aux
            sleep(Duration::from_secs(a));

            println!("Dron: incrementé mi posición, y ahora intentaré hacer publish");
            // Hace publish de su estado (de su current info) _ le servirá a otros drones para ver la condición b, y monitoreo para mostrarlo en mapa
            if let Ok(mut mqtt_client_l) = mqtt_client.lock() {
                println!("Dron: pude tomar lock"); // pero no pude []
                mqtt_client_l.mqtt_publish(AppsMqttTopics::DronTopic.to_str(), &self.current_info.to_bytes())?;
            };
        }

        // Al llegar, el dron ya no se encuentra en desplazamiento.
        self.unset_flying_info_values();

        Ok(())
    }

    /// Establece como `flying_info` a la dirección recibida, y a la velocidad leída del archivo de configuración.
    fn set_flying_info_values(&mut self, dir: (f64, f64)) {
        let speed = self.dron_properties.get_speed();
        let info = DronFlyingInfo::new(dir, speed);
        self.current_info.set_flying_info(info);
    }

    /// Establece `None` como `flying_info`, lo cual indica que el dron no está actualmente en desplazamiento.
    fn unset_flying_info_values(&mut self){
        self.current_info.unset_flying_info();
    }
}

#[cfg(test)]

mod test {
    use crate::apps::sist_dron::dron_state::DronState;

    use super::Dron;

    #[test]
    fn test_1_dron_se_inicia_con_id_y_estado_correctos() {
        let dron = Dron::new_internal(1).unwrap();

        assert_eq!(dron.current_info.get_id(), 1);
        assert_eq!(
            dron.current_info.get_state(),
            &DronState::ExpectingToRecvIncident
        ); // estado activo
    }

    #[test]
    fn test_2_dron_se_inicia_con_posicion_correcta() {
        let dron = Dron::new_internal(1).unwrap();

        // El dron inicia desde esta posición.
        // Aux, #ToDo: para que inicien desde su range center real, y no todos desde el mismo punto del mapa,
        //  aux: quizás sería necesario involucrar al id en la cuenta, ej una lat base + id*algún_factor, para espaciarlos en el mapa al iniciar. Ver [].
        assert_eq!(
            dron.current_info.get_current_position(),
            dron.dron_properties.get_range_center_position()
        );
    }

    #[test]
    fn test_3a_calculate_direction_da_la_direccion_esperada() {
        let dron = Dron::new_internal(1).unwrap();

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
        let dron = Dron::new_internal(1).unwrap();

        // Dados destino y origen
        let origin = dron.current_info.get_current_position(); // desde algo que no es el (0,0)
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
}
