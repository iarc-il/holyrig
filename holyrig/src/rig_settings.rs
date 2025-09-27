use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BaudRate {
    #[default]
    Baud1200,
    Baud2400,
    Baud4800,
    Baud9600,
    Baud19200,
    Baud38400,
    Baud57600,
    Baud115200,
}

impl Display for BaudRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = match self {
            BaudRate::Baud1200 => "1200",
            BaudRate::Baud2400 => "2400",
            BaudRate::Baud4800 => "4800",
            BaudRate::Baud9600 => "9600",
            BaudRate::Baud19200 => "19200",
            BaudRate::Baud38400 => "38400",
            BaudRate::Baud57600 => "57600",
            BaudRate::Baud115200 => "115200",
        };
        write!(f, "{result}")
    }
}

impl BaudRate {
    pub fn iter_rates() -> impl Iterator<Item = BaudRate> {
        [
            BaudRate::Baud1200,
            BaudRate::Baud2400,
            BaudRate::Baud4800,
            BaudRate::Baud9600,
            BaudRate::Baud19200,
            BaudRate::Baud38400,
            BaudRate::Baud57600,
            BaudRate::Baud115200,
        ]
        .into_iter()
    }
}

impl From<BaudRate> for u32 {
    fn from(value: BaudRate) -> Self {
        match value {
            BaudRate::Baud1200 => 1200,
            BaudRate::Baud2400 => 2400,
            BaudRate::Baud4800 => 4800,
            BaudRate::Baud9600 => 9600,
            BaudRate::Baud19200 => 19200,
            BaudRate::Baud38400 => 38400,
            BaudRate::Baud57600 => 57600,
            BaudRate::Baud115200 => 115200,
        }
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataBits {
    Bits5,
    Bits6,
    Bits7,
    #[default]
    Bits8,
}

impl Display for DataBits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = match self {
            DataBits::Bits5 => "5",
            DataBits::Bits6 => "6",
            DataBits::Bits7 => "7",
            DataBits::Bits8 => "8",
        };
        write!(f, "{result}")
    }
}

impl DataBits {
    pub fn iter_data_bits() -> impl Iterator<Item = DataBits> {
        [
            DataBits::Bits5,
            DataBits::Bits6,
            DataBits::Bits7,
            DataBits::Bits8,
        ]
        .into_iter()
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopBits {
    #[default]
    Bits1,
    Bits2,
}

impl Display for StopBits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = match self {
            StopBits::Bits1 => "1",
            StopBits::Bits2 => "2",
        };
        write!(f, "{result}")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RigSettings {
    pub id: usize,
    #[serde(default = "default_rig_type")]
    pub rig_type: String,
    pub port: String,
    pub baud_rate: BaudRate,
    pub data_bits: DataBits,
    pub parity: bool,
    pub stop_bits: StopBits,
    // true is high, false is low
    pub rts: bool,
    pub dtr: bool,
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u16,
    #[serde(default = "default_timeout")]
    pub timeout: u16,
}

fn default_rig_type() -> String {
    "unspecified".to_string()
}

fn default_poll_interval() -> u16 {
    500
}

fn default_timeout() -> u16 {
    1000
}

impl RigSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.rig_type == "unspecified" {
            return Err("Rig type must be specified".to_string());
        }

        if self.port.is_empty() {
            return Err("Serial port must be specified".to_string());
        }

        if !(100..=5000).contains(&self.poll_interval) {
            return Err("Poll interval must be between 100ms and 5000ms".to_string());
        }

        if !(100..=10000).contains(&self.timeout) {
            return Err("Timeout must be between 100ms and 10000ms".to_string());
        }

        Ok(())
    }

    pub fn with_id(mut self, id: usize) -> Self {
        self.id = id;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub rigs: Vec<RigSettings>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rigs: vec![Default::default()],
        }
    }
}

impl From<Vec<RigSettings>> for Settings {
    fn from(rigs: Vec<RigSettings>) -> Self {
        Self { rigs }
    }
}
