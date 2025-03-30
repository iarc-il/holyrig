use anyhow::Result;
use configparser::ini::Ini;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Command {
    pub command: Option<String>,
    pub reply_length: Option<u32>,
    pub reply_end: Option<String>,
    pub validate: Option<String>,
    pub value: Option<String>,
    pub values: HashMap<u32, String>,
    pub flags: HashMap<u32, String>,
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
    let mut parser = Ini::new();
    let mut config = parser.read(std::fs::read_to_string(path.as_ref())?).unwrap();
    let mut rig_description = RigDescription::new();

    println!("THE CONFIG:\n{config:#?}");
    for (section, prop) in &mut config {
        let mut command = Command {
            command: prop.remove("command").flatten(),
            reply_length: prop
                .remove("replyLength")
                .and_then(|s| s.map(|s| s.parse().unwrap())),
            reply_end: prop.remove("replyEnd").flatten(),
            validate: prop.remove("validate").flatten(),
            value: prop.remove("value").flatten(),
            values: HashMap::new(),
            flags: HashMap::new(),
        };

        for (key, value) in prop.iter() {
            let value = if let Some(value) = value {
                value.clone()
            } else {
                continue;
            };
            if let Some(idx) = key.strip_prefix("value") {
                if let Ok(n) = idx.parse::<u32>() {
                    let _ = command.values.insert(n, value);
                }
            } else if let Some(idx) = key.strip_prefix("flag") {
                if let Ok(n) = idx.parse::<u32>() {
                    let _ = command.flags.insert(n, value);
                }
            }
        }

        if section.starts_with("INIT") {
            rig_description.init_commands.push(command);
        } else if section.starts_with("STATUS") {
            rig_description.status_commands.push(command);
        } else {
            rig_description.param_commands.push(command);
        };
    }

    Ok(rig_description)
}
