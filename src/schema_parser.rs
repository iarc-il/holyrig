use serde::Deserialize;
use std::fmt::Display;
use std::path::Path;
use std::{collections::HashMap, fs};

#[derive(Debug)]
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
impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for ParseError {}

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
    #[serde(default)]
    pub params: Vec<(String, String)>,
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
