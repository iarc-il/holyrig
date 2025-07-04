use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct General {
    pub r#type: String,
    pub version: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CommandValidator {
    ReplyLength(u32),
    ReplyEnd(String),
    Mask(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandFormat {
    pub command: String,
    #[serde(flatten)]
    pub validator: Option<CommandValidator>,
    // pub reply_length: Option<u32>,
    // pub reply_end: Option<String>,
    // pub validate: Option<String>,
}

fn default_multiply() -> i32 {
    1
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandParam {
    pub index: u32,
    pub length: u32,
    pub format: String,
    #[serde(default)]
    pub add: i32,
    #[serde(default  = "default_multiply")]
    pub multiply: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RigFile {
    pub general: General,
    pub init: HashMap<String, CommandFormat>,
    pub commands: HashMap<String, CommandFormat>,
    pub status: HashMap<String, CommandFormat>,
}

impl RigFile {
    pub fn new() -> Self {
        Self {
            general: General {
                r#type: "transceiver".to_string(),
                version: 1,
            },
            init: HashMap::new(),
            commands: HashMap::new(),
            status: HashMap::new(),
        }
    }
}

impl Default for RigFile {
    fn default() -> Self {
        Self::new()
    }
}
