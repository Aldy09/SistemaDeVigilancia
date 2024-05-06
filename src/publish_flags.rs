use std::io::{Error, ErrorKind};

#[derive(Debug, PartialEq)]
/// Flags para el mensaje Publish.
/// Los flags son:
///  - type = 3 siempre, para el mensaje publish _ 4 bits
///  - dup flag = 0 o 1 _ 1 bit
///  - qos level = {00, 01, 10} _ 2 bits _ el 11 está prohibido.
///  - retain = 0 o 1 _ 1 bit.
pub struct PublishFlags {
    dup: u8,    // 0: no duplicado, 1: duplicado
    qos: u8,    // 0: at most once, 1: at least once, 2: exactly once
    retain: u8, // 0: no retain, 1: retain
}

impl PublishFlags {
    /// Recibe todos los valores de flags para mensaje publish excepto el tipo que siempre vale 3.
    /// Devuelve un struct PublishFlags creado, que contiene el byte de flags.
    pub fn new(dup: u8, qos: u8, retain: u8) -> Result<Self, Error> {
        if dup > 1 || qos > 2 || retain > 1 {
            return Err(Error::new(ErrorKind::Other, "Flags para publish inválidos"));
        }
        Ok(PublishFlags { dup, qos, retain })
    }

    //Chequea si qos > 0
    pub fn is_qos_greater_than_0(&self) -> bool {
        self.qos > 0
    }

    pub fn to_flags_byte(&self) -> u8 {
        let mut byte_de_flags: u8 = 0;

        byte_de_flags |= self.dup << 3; // dup se coloca en el bit 3
        byte_de_flags |= self.qos << 1; // qos se coloca en los bits 1 y 2
        byte_de_flags |= self.retain; // retain se coloca en el bit 0

        byte_de_flags
    }
    pub fn from_flags_byte(byte_de_flags: u8) -> Result<PublishFlags, Error> {
        let retain = byte_de_flags & 0b0000_0001;
        let qos = (byte_de_flags & 0b0000_0110) >> 1; // desplaza los bits a la derecha para que qos sea un u8 entre 0 y 3.
        let dup = (byte_de_flags & 0b0000_1000) >> 3; // desplaza los bits a la derecha para que dup sea un u8 entre 0 y 1.

        Ok(PublishFlags { dup, qos, retain })
    }
    /*
    pub fn from_flags_byte(byte_de_flags: &u8) -> Result<PublishFlags, Error> {
        let retain = byte_de_flags & 0b0000_0001;
        let mut qos = byte_de_flags & 0b0000_0110;
        qos >>= 1; // para eliminar el bit de la derecha, y quedarme solo con dos bits.
        let mut dup = byte_de_flags & 0b0000_1000;
        dup >>= 3;
        let mut tipo = byte_de_flags & 0b1111_0000;
        tipo >>= 4;

        if tipo != 3 {
            return Err(Error::new(ErrorKind::Other, "Flags para publish leídos con tipo inválido."));
        }

        Ok(PublishFlags{dup, qos, retain})
    }
    */
}
/*
#[cfg(test)]
mod test {
    use super::PublishFlags;

    #[test]
    fn test_1_intentar_crear_pub_flags_inválidos_da_error() {
        // dup debe ser 0 o 1; mayor da error
        let flags_invalidos_a = PublishFlags::new(2, 0, 0);
        // retain debe ser 0 o 1; mayor da error
        let flags_invalidos_b = PublishFlags::new(0, 0, 2);
        // qos debe ser 0, 1, o 2; mayor da error
        let flags_invalidos_c = PublishFlags::new(0, 3, 0);

        assert!(flags_invalidos_a.is_err());
        assert!(flags_invalidos_b.is_err());
        assert!(flags_invalidos_c.is_err());
    }

    #[test]
    fn test_2_publish_flags_se_pasa_a_bytes_correctamente() {
        let flags = PublishFlags::new(0, 2, 1).unwrap();
        // byte debería ser | 0111 | 0 | 10 | 1 |,
        // es decir:        | 0111 0101 |

        let byte_esperado: u8 = 0b0011_0101;

        assert_eq!(flags.to_flags_byte(), byte_esperado);
    }

    #[test]
    fn test_3_publish_flags_se_pasa_a_bytes_y_se_interpreta_correctamente() {
        let flags = PublishFlags::new(0, 2, 1).unwrap();

        let byte_de_flags = flags.to_flags_byte();

        let flags_reconstruido = PublishFlags::from_flags_byte(&byte_de_flags);

        assert!(flags_reconstruido.is_ok());
        assert_eq!(flags_reconstruido.unwrap(), flags);
    }

    #[test]
    fn test_4_byte_de_flags_ilegal_da_error() {
        // Es malformado si el tipo no es 3, ie si los 4 bits más significativos no son '0111'
        let byte_de_flags_malformado_a: u8 = 0b1010_0000;
        let byte_de_flags_malformado_b: u8 = 0b0010_0000;

        let flags_reconstruido_a = PublishFlags::from_flags_byte(&byte_de_flags_malformado_a);
        let flags_reconstruido_b = PublishFlags::from_flags_byte(&byte_de_flags_malformado_b);

        assert!(flags_reconstruido_a.is_err());
        assert!(flags_reconstruido_b.is_err());
    }


}
*/
