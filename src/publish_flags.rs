// Archivo en construcción, actualmente con lo necesario para que compile y funcione.
#[derive(Debug, PartialEq)]
/// Flags para el mensaje Publish.
/// Los flags son:
///  - type = 3 siempre, para el mensaje publish _ 4 bits
///  - dup flag = 0 o 1 _ 1 bit
///  - qos level = {00, 01, 10} _ 2 bits
///  - retain = 0 o 1 _ 1 bit.
pub struct PublishFlags {
    //byte_de_flags: u8,
    dup: u8,
    qos: u8,
    retain: u8,
}
impl PublishFlags {
    /// Recibe todos los valores de flags para mensaje publish excepto el tipo que siempre vale 3.
    /// Devuelve un struct PublishFlags creado, que contiene el byte de flags.
    pub fn new(dup: u8, qos: u8, retain: u8) -> Self {
        PublishFlags { dup, qos, retain  }
    }
}

impl PublishFlags {
    pub fn to_flags_byte(&self) -> u8 {
        let mut byte_de_flags: u8 = 0;

        let tipo = 3 << 4; // tipo siempre vale 3
        byte_de_flags |= tipo;

        byte_de_flags |= self.dup << 3;

        byte_de_flags |= self.qos << 1;

        byte_de_flags |= self.retain;

        byte_de_flags        
    }
}

#[cfg(test)]
mod test {
    use super::PublishFlags;

    #[test]
    fn test_1_publish_flags_se_pasa_a_bytes_correctamente() {
        let flags = PublishFlags::new(0, 2, 1);
        // byte debería ser | 0111 | 0 | 10 | 1 |,
        // es decir:        | 0111 0101 |

        let byte_esperado: u8 = 0b0011_0101;
                            // aux: 5+16+32+64=80+37=117
        // da left 53:  = 32+16+5
        // 0011_0101, o sea falla el segundo de izq a derecha. El type.
        // Aux:
        // 0000_0011 = 3
        // ah es que está perfecto, un tres es 11, y no 111 ;D. Bien.

        assert_eq!(flags.to_flags_byte(), byte_esperado);
    }
}