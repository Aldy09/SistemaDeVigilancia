#[derive(Debug)]
#[allow(dead_code)]
pub struct ConnectFlags {
    clean_session: bool,
    will_flag: bool,
    will_qos: u8,
    will_retain: bool,
    user_name_flag: bool,
    password_flag: bool,
}
#[allow(dead_code)]
impl ConnectFlags {
    pub fn new(clean_session: bool, will_flag: bool, will_qos: u8, will_retain: bool, user_name_flag: bool, password_flag: bool) -> Self {
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
            flags_byte |= 0b0000_0010;
        }

        if self.will_flag {
            flags_byte |= 0b0000_0100;
            flags_byte |= (self.will_qos << 3) & 0b0001_1000;
            if self.will_retain {
                flags_byte |= 0b0010_0000; 
            }
        }

        if self.user_name_flag {
            flags_byte |= 0b1000_0000; 
        }

        if self.password_flag {
            flags_byte |= 0b0100_0000; 
        }

        flags_byte
    }
}