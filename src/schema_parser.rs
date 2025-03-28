use serde::Deserialize;
use std::path::Path;
use std::{collections::HashMap, fs};

pub enum ParseError {
    IO(std::io::Error),
    TOML(toml::de::Error),
}
impl From<std::io::Error> for ParseError {
    fn from(value: std::io::Error) -> Self {
        ParseError::IO(value)
    }
}
impl From<toml::de::Error> for ParseError {
    fn from(value: toml::de::Error) -> Self {
        ParseError::TOML(value)
    }
}

#[derive(Debug, Deserialize)]
pub struct General {
    pub rig_type: String,
}

#[derive(Debug, Deserialize)]
pub struct Enum {
    pub members: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Command {
    pub params: Option<Vec<(String, String)>>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub general: General,
    pub enums: HashMap<String, Enum>,
    pub commands: HashMap<String, Command>,
}

pub fn parse_schema_file<P: AsRef<Path>>(path: P) -> Result<Config, ParseError> {
    let toml_content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&toml_content)?;
    Ok(config)
}
