#[derive(Debug)]
pub struct Payload<'a> {
    pub client_id: &'a str,
    pub will_topic: Option<&'a str>,
    pub will_message: Option<&'a str>,
    pub username: Option<&'a str>,
    pub password: Option<&'a str>,
}