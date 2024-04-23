#[derive(Debug)]
struct ConnectFlags {
    clean_session: bool,
    will_flag: bool,
    will_qos: u8, // Puede ser 0, 1, o 2
    will_retain: bool,
    user_name_flag: bool,
    password_flag: bool,
}

impl ConnectFlags {
    fn new(clean_session: bool, will_flag: bool, will_qos: u8, will_retain: bool, user_name_flag: bool, password_flag: bool) -> Self {
        ConnectFlags {
            clean_session,
            will_flag,
            will_qos,
            will_retain,
            user_name_flag,
            password_flag,
        }
    }

    fn to_flags_byte(&self) -> u8 {
        let mut flags_byte: u8 = 0;

        if self.clean_session {
            flags_byte |= 0b0000_0010; // Setea el bit 1 (Clean Session)
        }

        if self.will_flag {
            flags_byte |= 0b0000_0100; // Setea el bit 2 (Will Flag)
            flags_byte |= (self.will_qos << 3) & 0b0001_1000; // Ajusta los bits 3 y 4 (Will QoS)
            if self.will_retain {
                flags_byte |= 0b0010_0000; // Setea el bit 5 (Will Retain)
            }
        }

        if self.user_name_flag {
            flags_byte |= 0b1000_0000; // Setea el bit 7 (User Name Flag)
        }

        if self.password_flag {
            flags_byte |= 0b0100_0000; // Setea el bit 6 (Password Flag)
        }

        flags_byte
    }
}