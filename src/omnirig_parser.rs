use anyhow::{bail, Result};
use configparser::ini::Ini;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum EndOfData {
    Length(u32),
    String(String),
}

#[derive(Debug, Clone)]
pub struct Command {
    pub command: String,
    pub end_of_data: EndOfData,
    pub validate: Option<String>,
    pub value: Option<String>,
    pub values: Vec<String>,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RigDescription {
    pub init_commands: Vec<Command>,
    pub param_commands: Vec<Command>,
    pub status_commands: Vec<Command>,
}

impl RigDescription {
    fn new() -> Self {
        Self {
            init_commands: vec![],
            param_commands: vec![],
            status_commands: vec![],
        }
    }
}

pub fn parse_ini_file<P: AsRef<Path>>(path: P) -> Result<RigDescription> {
    let raw_ini = std::fs::read_to_string(path.as_ref())?;
    parse_ini_data(raw_ini)
}

pub fn parse_ini_data(ini_data: String) -> Result<RigDescription> {
    let mut parser = Ini::new();
    let mut config = parser.read(ini_data).unwrap();
    let mut rig_description = RigDescription::new();

    for (section, prop) in &mut config {
        let command = if let Some(command) = prop.remove("command").flatten() {
            command
        } else {
            continue;
        };

        let reply_end = prop.remove("replyend").flatten();
        let reply_length = prop.remove("replylength").flatten();
        let end_of_data = match (reply_end, reply_length) {
            (None, Some(length)) => EndOfData::Length(length.parse()?),
            (Some(string), None) => EndOfData::String(string),
            (Some(_), Some(_)) => {
                bail!("Cannot have both ReplyEnd and replyLength fields in section {section}");
            }
            (None, None) => {
                bail!("Missing ReplyEnd or replyLength fields in section {section}");
            }
        };

        let validate = prop.remove("validate").flatten();
        let value = prop.remove("value").flatten();

        let mut command = Command {
            command,
            end_of_data,
            validate,
            value,
            values: vec![],
            flags: vec![],
        };

        let mut flags = vec![];
        let mut values = vec![];

        for (key, value) in prop.iter() {
            let value = if let Some(value) = value {
                value.clone()
            } else {
                continue;
            };
            if let Some(index) = key.strip_prefix("value") {
                if let Ok(index) = index.parse::<u8>() {
                    values.push((index, value));
                }
            } else if let Some(index) = key.strip_prefix("flag") {
                if let Ok(index) = index.parse::<u8>() {
                    flags.push((index, value));
                }
            }
        }

        flags.sort();
        values.sort();

        command.flags = flags.into_iter().map(|(_, value)| value).collect();
        command.values = values.into_iter().map(|(_, value)| value).collect();

        if section.to_uppercase().starts_with("INIT") {
            rig_description.init_commands.push(command);
        } else if section.to_uppercase().starts_with("STATUS") {
            rig_description.status_commands.push(command);
        } else {
            rig_description.param_commands.push(command);
        };
    }

    Ok(rig_description)
}
