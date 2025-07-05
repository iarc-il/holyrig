use std::fmt::Display;

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum RigType {
    #[default]
    Unspecified,
    IC7300,
    FT891,
}

impl Display for RigType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted = match self {
            RigType::Unspecified => "Insert rig type...",
            RigType::IC7300 => "IC7300",
            RigType::FT891 => "FT891",
        };
        write!(f, "{formatted}")
    }
}

#[derive(Debug, Clone)]
pub struct RigSettings {
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

impl Default for RigSettings {
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
