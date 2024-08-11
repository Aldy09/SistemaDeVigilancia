/// Motivo por el cual user se desconectó del servidor,
/// Puede ser voluntaria si se recibió un mensaje DisconnectMessage,
/// o involuntaria si se dejó de recibir por el stream (ej problemas en la conexión a internet).
pub enum DisconnectReason {
    Voluntaria, // Aux: suerte para poner algo en inglés acá, ja, traducir esto.
    Involuntaria,
}