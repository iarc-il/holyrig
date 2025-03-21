#[derive(Debug, Default)]
pub enum RigType {
    #[default]
    Unspecified,
    IC7300,
    FT891,
}

pub struct Rig {
    pub rig_type: RigType,
    pub port: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub parity: bool,
    pub stop_bits: u8,
    // true is high, false is low
    pub rts: bool,
    pub dtr: bool,
    pub poll_interval: u16,
    pub timeout: u16,
}

impl Default for Rig {
    fn default() -> Self {
        Self {
            rig_type: RigType::Unspecified,
            port: String::new(),
            baud_rate: 0,
            data_bits: 0,
            parity: false,
            stop_bits: 0,
            rts: false,
            dtr: false,
            poll_interval: 0,
            timeout: 0,
        }
    }
}
