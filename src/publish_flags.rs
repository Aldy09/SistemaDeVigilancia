// Archivo en construcción, actualmente con lo necesario para que compile y funcione.
#[derive(Debug, PartialEq)]
pub struct PublishFlags {

}
impl PublishFlags {
    pub fn new() -> Self {

        PublishFlags {  }
    }
}
impl PublishFlags {
    pub fn to_bytes(&self) -> Vec<u8> { // [] ver
        vec![0] // probando, para que compile
    }
}