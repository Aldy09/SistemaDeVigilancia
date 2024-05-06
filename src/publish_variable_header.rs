#[derive(Debug)]
pub struct VariableHeader {
    //byte1:long name bits mas signif,
    //byte2:long name bits menos signif
    pub topic_name: String, // Cambiado de &'a str a String //bytes 1-5,(ejemplo topic_name: "a/b")

    pub packet_identifier: Option<u16>, // bytes 6-7, solo si qos > 0 ,1 o 2
}
