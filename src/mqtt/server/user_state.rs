/// Representa el estado de un `User` del MQTTServer:
/// - Active indica que no se detectaron problemas con la conexión con dicho User,
/// - TemporallyDisconnected indica que el User terminó abruptamente la conexión (ie hizo ctrl+C) y podría reconectarse en el futuro.
#[derive(Debug, PartialEq)]
pub enum UserState {
    Active,
    TemporallyDisconnected
}