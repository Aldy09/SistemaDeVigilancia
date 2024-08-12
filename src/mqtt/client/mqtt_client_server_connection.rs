use std::net::{SocketAddr, TcpStream};

use std::io::{self, Error, ErrorKind, Read};
use std::time::Duration;

use crate::mqtt::messages::{
    connack_message::ConnackMessage, connect_message::ConnectMessage,
    connect_return_code::ConnectReturnCode, packet_type::PacketType,
};
use crate::mqtt::mqtt_utils::fixed_header::FixedHeader;
use crate::mqtt::mqtt_utils::utils::{
    get_fixed_header_from_stream_for_conn, get_whole_message_in_bytes_from_stream,
    write_message_to_stream,
};
use crate::mqtt::mqtt_utils::will_message_utils::will_message::WillMessageData;
use crate::mqtt::stream_type::StreamType;

use super::mqtt_client::ClientStreamType;

pub struct MqttClientConnection {
    stream: ClientStreamType,
}

impl MqttClientConnection {
    pub fn mqtt_connect_to_broker(
        client_id: String,
        addr: &SocketAddr,
        will: Option<WillMessageData>,
    ) -> Result<ClientStreamType, Error> {
        //pub fn mqtt_connect_to_broker(client_id: String, addr: &SocketAddr, will: Option<WillMessageData>) -> Result<(ClientStreamType, ConnectMessage), Error> {
        //will_msg_content: String, will_topic: String, will_qos: u8
        //will.get_will_msg_content(), will.get_will_topic(), will.get_qos()
        //let will_topic = String::from("desc"); // PROBANDO
        //let will_qos = 1; // PROBANDO, ESTO VA POR PARÁMETRO
        // Inicializaciones
        // Intenta conectar al servidor MQTT
        let mut stream = TcpStream::connect(addr)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "error del servidor"))?;
        let mut connection = Self { stream: stream.try_clone()? }; // Aux: probando.

        // Aux: sintaxis es let (a, b) = if condicion { (a_si_true, b_si_true) } else { (a_si_false, b_si_false) };
        let (will_msg_content, will_topic, will_qos, _will_retain) = if let Some(will) = will {
            (
                Some(will.get_will_msg_content()),
                Some(will.get_will_topic()),
                will.get_qos(),
                will.get_will_retain(),
            )
        } else {
            (None, None, 0, 0) // aux: decidir / revisar, asumimos 0 si no están? []
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

        // Iniciando nuevo
        connection.send_and_retransmit(&mut msg)?;
        // Fin nuevo

        println!("Mqtt cliente leyendo: esperando connack.");

        Ok(stream)

        //Ok((stream, msg))
    }

    /// Función para ser usada por `MQTTClient`, cuando el `Retransmitter` haya determinado que el `msg` debe
    /// enviarse por el stream a server.
    fn send_msg(&mut self, bytes_msg: Vec<u8>) -> Result<(), Error> {
        write_message_to_stream(&bytes_msg, &mut self.stream)?;
        Ok(())
    }

    /// Envía el mensaje `msg` recibido una vez, espera por el ack, y si es necesario lo retransmite una cierta
    /// cantidad de veces.
    fn send_and_retransmit(&mut self, msg: &mut ConnectMessage) -> Result<(), Error> {
        self.send_msg(msg.to_bytes())?;
        println!("Envía connect: \n   {:?}", msg);
        if let Err(e) = self.wait_for_connack_and_retransmit(msg) {
            println!("Error al esperar ack del publish: {:?}", e);
        };
        Ok(())
    }

    /// Espera a recibir el ack para el packet_id del mensaje `msg`, si no lo recibe, retransmite.
    fn wait_for_connack_and_retransmit(&mut self, msg: &mut ConnectMessage) -> Result<(), Error> {
        // Espero la primera vez, para el publish que hicimos arriba. Si se recibió ack, no hay que hacer nada más.
        let mut received_ack = self.has_connack_arrived()?;
        if received_ack {
            return Ok(());
        }

        // No recibí ack, entonces tengo que continuar retransmitiendo, hasta un máx de veces.
        const AMOUNT_OF_RETRIES: u8 = 5; // cant de veces que va a reintentar, hasta que desista y dé error.
        let mut remaining_retries = AMOUNT_OF_RETRIES;

        while !received_ack || remaining_retries > 0 {
            // Si el Retransmitter determina que se debe volver a enviar el mensaje, lo envío.
            
            self.send_msg(msg.to_bytes())?;          

            received_ack = self.has_connack_arrived()?;

            remaining_retries -= 1;
        }

        if !received_ack {
            // Ya salí del while, retransmití muchas veces y nunca recibí el ack, desisto.
            return Err(Error::new(
                ErrorKind::Other,
                "MAXRETRIES, se retransmitió sin éxito.",
            ));
        }

        Ok(())
    }

    /// Lee una vez, con timeout, para esperar recibir el ack en a lo sumo una cierta cantidad de tiempo.
    /// Retorna Ok de si le llegó el connack.
    fn has_connack_arrived(&mut self) -> Result<bool, Error> {
        //-> Result<([u8; 2], FixedHeader), Error> {
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
                    // Continúo leyendo y procesando el connack
                    self.read_connack(fixed_header_buf, fixed_header); // Aux: revisar esta parte, xq ya se hace adentro.
                    return Ok(true);
                } else {
                    return Err(Error::new(ErrorKind::Other, "Se lee pero es un connack"));
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut {
                    // Este tipo de error es especial de timeout, significa que pasó el tiempo y no llegó el connack
                    return Ok(false);
                } else {
                    // Éste es un error real
                    println!("Error al leer: {:?}", e);
                    return Err(Error::new(ErrorKind::Other, "Error al leer."));
                }
            }
        }
    }

    /// Recibe un fixed header y verifica que haya sido de tipo Connack
    fn read_connack(
        &mut self,
        fixed_header_buf: [u8; 2],
        fixed_header: FixedHeader,
    ) -> Result<(), Error> {
        let fixed_header_info = (fixed_header_buf, fixed_header);

        // Verifica que haya sido de tipo Connack
        let recvd_msg_type = fixed_header_info.1.get_message_type();
        if recvd_msg_type == PacketType::Connack {
            // ConnAck
            println!("Mqtt cliente leyendo: recibo conn ack");
            let recvd_bytes = get_whole_message_in_bytes_from_stream(
                &fixed_header_info.1,
                &mut self.stream,
                &fixed_header_info.0,
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
        } else {
            // No debería darse
            Err(Error::new(
                ErrorKind::InvalidData,
                "Error: Mensaje recibido pero no es Connack.",
            ))
        }
    }
}
