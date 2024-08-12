use std::net::{SocketAddr, TcpStream};

use std::io::{self, Error, ErrorKind, Read};
use std::time::Duration;

use crate::logging::string_logger::StringLogger;
use crate::mqtt::messages::{
    connack_message::ConnackMessage, connect_message::ConnectMessage,
    connect_return_code::ConnectReturnCode, packet_type::PacketType,
};
use crate::mqtt::mqtt_utils::fixed_header::FixedHeader;
use crate::mqtt::mqtt_utils::utils::{
    get_whole_message_in_bytes_from_stream, write_message_to_stream,
};
use crate::mqtt::mqtt_utils::will_message_utils::will_message::WillMessageData;

use super::mqtt_client::ClientStreamType;

pub struct MqttClientConnector {
    stream: ClientStreamType,
    logger: StringLogger,
}

impl MqttClientConnector {
    pub fn mqtt_connect_to_broker(
        client_id: String,
        addr: &SocketAddr,
        will: Option<WillMessageData>,
        logger: StringLogger,
    ) -> Result<ClientStreamType, Error> {
        // Intenta conectar al servidor MQTT
        let stream = TcpStream::connect(addr)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Error para establecer conexión con servidor."))?;
        let mut connector = Self {
            stream: stream.try_clone()?, // obs: como no devuelvo Self, esta copia del stream se dropea al salir de esta función y no molesta.
            logger,
        };

        // Aux: sintaxis es let (a, b) = if condicion { (a_si_true, b_si_true) } else { (a_si_false, b_si_false) };
        let (will_msg_content, will_topic, will_qos, _will_retain) = if let Some(will) = will {
            (
                Some(will.get_will_msg_content()),
                Some(will.get_will_topic()),
                will.get_qos(),
                will.get_will_retain(),
            )
        } else {
            (None, None, 1, 1)
        };

        // Crea el mensaje tipo Connect y lo pasa a bytes
        let mut msg = ConnectMessage::new(
            client_id,
            will_topic,
            will_msg_content,
            Some("usuario0".to_string()),
            Some("rustx123".to_string()),
            will_qos,
        );

        connector.logger.log("Mqtt: Enviando connect msg.".to_string());
        connector.send_and_retransmit(&mut msg)?;
        connector.logger.log("Mqtt: connack recibido.".to_string());

        Ok(stream)
    }
    
    /// Envía el mensaje `msg` recibido una vez, espera por el ack, y si es necesario lo retransmite una cierta
    /// cantidad de veces.
    fn send_and_retransmit(&mut self, msg: &mut ConnectMessage) -> Result<(), Error> {
        self.send_msg(msg.to_bytes())?;
        self.wait_for_connack_and_retransmit(msg)?;        
        Ok(())
    }
    
    /// Función para ser usada por `MQTTClient`, cuando el `Retransmitter` haya determinado que el `msg` debe
    /// enviarse por el stream a server.
    fn send_msg(&mut self, bytes_msg: Vec<u8>) -> Result<(), Error> {
        write_message_to_stream(&bytes_msg, &mut self.stream)?;
        Ok(())
    }
    
    /// Espera a recibir el ack para el mensaje `msg`, si no lo recibe, retransmite.
    fn wait_for_connack_and_retransmit(&mut self, msg: &mut ConnectMessage) -> Result<(), Error> {
        // Espero la primera vez, para el connect que hicimos arriba. Si se recibió ack, no hay que hacer nada más.
        let mut received_ack = self.has_connack_arrived()?;
        if received_ack {
            return Ok(());
        }

        // No recibí ack, entonces tengo que continuar retransmitiendo, hasta un máx de veces.
        const AMOUNT_OF_RETRIES: u8 = 5; // cant de veces que va a reintentar, hasta que desista y dé error.
        let mut remaining_retries = AMOUNT_OF_RETRIES;

        while !received_ack && remaining_retries > 0 {
            // Lo vuelvo a enviar y a verificar si recibo ack
            self.send_msg(msg.to_bytes())?;
            received_ack = self.has_connack_arrived()?;
            self.logger.log("Mqtt: Retransmitiendo...".to_string());

            remaining_retries -= 1;
        }

        if !received_ack {
            // Ya salí del while, retransmití muchas veces y nunca recibí el ack, desisto.
            return Err(Error::new(
                ErrorKind::Other,
                "MAXRETRIES, se retransmitió el connect sin éxito.",
            ));
        }

        Ok(())
    }

    /// Lee una vez, con timeout, para esperar recibir el ack en a lo sumo una cierta cantidad de tiempo.
    /// Retorna Ok de si le llegó el connack.
    fn has_connack_arrived(&mut self) -> Result<bool, Error> {
        const FIXED_HEADER_LEN: usize = FixedHeader::fixed_header_len();
        let mut fixed_header_buf: [u8; 2] = [0; FIXED_HEADER_LEN];

        // Espero recibir un connack en como mucho un cierto tiempo constante.
        const ACK_WAITING_INTERVAL: u64 = 1000;
        let max_waiting_interval = Duration::from_millis(ACK_WAITING_INTERVAL);
        self.stream.set_read_timeout(Some(max_waiting_interval))?;
        // Leo
        let was_there_connack = self.stream.read(&mut fixed_header_buf);
        match was_there_connack {
            Ok(_) => {
                // He leído bytes de un fixed_header, tengo que ver de qué tipo es.
                let fixed_header = FixedHeader::from_bytes(fixed_header_buf.to_vec());
                if fixed_header.get_message_type() == PacketType::Connack {
                    // Unset del timeout, ya que como hubo fixed header de connack,
                    // es 100% seguro que seguirá el resto del mensaje
                    self.stream.set_read_timeout(None)?;
                    // Continúo leyendo el Connack, devuelvo error si la conexión no fue aceptada por el server
                    self.complete_connack_read_and_analyze_it(fixed_header_buf, fixed_header)?;
                    Ok(true)
                } else {
                    // No sebería darse
                    Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Error: se lee pero es un connack",
                    ))
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut {
                    // Este tipo de error es especial de timeout, significa que pasó el tiempo y no llegó el connack
                    Ok(false)
                } else {
                    // Éste es un error real
                    println!("Error al leer: {:?}", e);
                    Err(Error::new(ErrorKind::Other, "Error al leer."))
                }
            }
        }
    }

    /// Recibe un fixed header de un mensaje de tipo Connack, y completa su lectura.
    /// Analiza si la conexión fue (Ok) o no (Error) aceptada por el servidor.
    fn complete_connack_read_and_analyze_it(
        &mut self,
        fixed_header_buf: [u8; 2],
        fixed_header: FixedHeader,
    ) -> Result<(), Error> {
        // ConnAck
        println!("Mqtt cliente leyendo: recibo conn ack");
        let recvd_bytes = get_whole_message_in_bytes_from_stream(
            &fixed_header,
            &mut self.stream,
            &fixed_header_buf,
        )?;
        // Entonces tengo el mensaje completo
        let msg = ConnackMessage::from_bytes(&recvd_bytes)?; //
        println!("   Mensaje conn ack completo recibido: {:?}", msg);
        let ret = msg.get_connect_return_code();
        if ret == ConnectReturnCode::ConnectionAccepted {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::InvalidData,
                "La conexión no fue aceptada.",
            ))
        }
    }
}
