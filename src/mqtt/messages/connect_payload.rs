#[derive(Debug, PartialEq)]
pub struct Payload {
    pub client_id: String,
    pub will_topic: Option<String>,
    pub will_message: Option<String>, // de estar presente, se manda la len y la string.
    pub username: Option<String>,
    pub password: Option<String>,
}
