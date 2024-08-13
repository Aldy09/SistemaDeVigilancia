use chrono::Local;

/// Encargado de proporcionar timestamp.
#[derive(Debug)]
pub struct Time {}

impl Time {
    /// Devuelve un timestamp actual, parseado a string de la forma `[%d/%m/%Y %H:%M:%S]`.`
    pub fn now_as_string() -> String {

        let datetime = Local::now();
        let string_timestamp = datetime.format("[%d/%m/%Y %H:%M:%S]").to_string();

        string_timestamp
    }
}
