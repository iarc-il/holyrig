use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub enum SchemaError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    InvalidRigType(String),
    InvalidVersion(u32),
    EmptyEnum(String),
    DuplicateEnumMember {
        enum_name: String,
        member: String,
    },
    UndefinedType {
        command: String,
        param: String,
        type_name: String,
    },
    DuplicateParameter {
        command: String,
        param: String,
    },
}

impl std::error::Error for SchemaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SchemaError::Io(err) => Some(err),
            SchemaError::Toml(err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaError::Io(err) => write!(f, "IO error: {err}"),
            SchemaError::Toml(err) => write!(f, "TOML parsing error: {err}"),
            SchemaError::InvalidRigType(ty) => write!(f, "Invalid rig_type: {ty}"),
            SchemaError::InvalidVersion(ver) => write!(f, "Invalid version: {ver}"),
            SchemaError::EmptyEnum(name) => write!(f, "Enum {name} has no members"),
            SchemaError::DuplicateEnumMember { enum_name, member } => {
                write!(f, "Duplicate member {member} in enum {enum_name}")
            }
            SchemaError::UndefinedType {
                command,
                param,
                type_name,
            } => {
                write!(
                    f,
                    "Command {command} parameter {param} has undefined type {type_name}"
                )
            }
            SchemaError::DuplicateParameter { command, param } => {
                write!(f, "Duplicate parameter name {param} in command {command}")
            }
        }
    }
}

impl From<std::io::Error> for SchemaError {
    fn from(err: std::io::Error) -> Self {
        SchemaError::Io(err)
    }
}

impl From<toml::de::Error> for SchemaError {
    fn from(err: toml::de::Error) -> Self {
        SchemaError::Toml(err)
    }
}

#[derive(Debug, Deserialize)]
pub struct General {
    pub rig_type: String,
    pub version: u32,
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
pub struct Schema {
    pub general: General,
    #[serde(default)]
    pub enums: HashMap<String, Enum>,
    pub commands: HashMap<String, Command>,
}

impl Schema {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, SchemaError> {
        let content = std::fs::read_to_string(path)?;
        let schema: Schema = toml::from_str(&content)?;
        schema.validate()?;
        Ok(schema)
    }

    pub fn validate(&self) -> Result<(), SchemaError> {
        if self.general.rig_type != "transceiver" {
            return Err(SchemaError::InvalidRigType(self.general.rig_type.clone()));
        }
        if self.general.version != 1 {
            return Err(SchemaError::InvalidVersion(self.general.version));
        }

        for (enum_name, enum_def) in &self.enums {
            if enum_def.members.is_empty() {
                return Err(SchemaError::EmptyEnum(enum_name.clone()));
            }
            let mut seen = std::collections::HashSet::new();
            for member in &enum_def.members {
                if !seen.insert(member) {
                    return Err(SchemaError::DuplicateEnumMember {
                        enum_name: enum_name.clone(),
                        member: member.clone(),
                    });
                }
            }
        }

        for (cmd_name, cmd) in &self.commands {
            for (param_name, param_type) in &cmd.params {
                match param_type.as_str() {
                    "int" | "bool" => {}
                    type_name => {
                        if !self.enums.contains_key(type_name) {
                            return Err(SchemaError::UndefinedType {
                                command: cmd_name.clone(),
                                param: param_name.clone(),
                                type_name: type_name.to_string(),
                            });
                        }
                    }
                }
            }

            let mut seen = std::collections::HashSet::new();
            for (param_name, _) in &cmd.params {
                if !seen.insert(param_name) {
                    return Err(SchemaError::DuplicateParameter {
                        command: cmd_name.clone(),
                        param: param_name.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    pub fn get_command(&self, name: &str) -> Option<&Command> {
        self.commands.get(name)
    }

    pub fn get_enum(&self, name: &str) -> Option<&Enum> {
        self.enums.get(name)
    }

    pub fn is_valid_enum_value(&self, enum_name: &str, value: &str) -> bool {
        self.get_enum(enum_name)
            .map(|e| e.members.iter().any(|m| m == value))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_schema() {
        let schema_str = r#"
            [general]
            rig_type = "transceiver"
            version = 1

            [enums.vfo]
            members = ["A", "B", "current"]

            [commands.set_freq]
            params = [
                ["freq", "int"],
                ["target", "vfo"]
            ]
        "#;

        let schema: Schema = toml::from_str(schema_str).unwrap();
        assert!(schema.validate().is_ok());
    }

    #[test]
    fn test_invalid_rig_type() {
        let schema_str = r#"
            [general]
            rig_type = "invalid"
            version = 1

            [commands.set_freq]
            params = [
                ["freq", "int"],
                ["target", "vfo"]
            ]
        "#;

        let schema: Schema = toml::from_str(schema_str).unwrap();
        match schema.validate() {
            Err(SchemaError::InvalidRigType(ty)) => assert_eq!(ty, "invalid"),
            other => panic!("Expected InvalidRigType error, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_version() {
        let schema_str = r#"
            [general]
            rig_type = "transceiver"
            version = 2

            [commands.set_freq]
            params = [
                ["freq", "int"],
                ["target", "vfo"]
            ]
        "#;

        let schema: Schema = toml::from_str(schema_str).unwrap();
        match schema.validate() {
            Err(SchemaError::InvalidVersion(ver)) => assert_eq!(ver, 2),
            other => panic!("Expected InvalidVersion error, got {other:?}"),
        }
    }

    #[test]
    fn test_duplicate_enum_member() {
        let schema_str = r#"
            [general]
            rig_type = "transceiver"
            version = 1

            [enums.vfo]
            members = ["A", "A", "B"]

            [commands.set_freq]
            params = [
                ["freq", "int"],
                ["target", "vfo"]
            ]
        "#;

        let schema: Schema = toml::from_str(schema_str).unwrap();
        match schema.validate() {
            Err(SchemaError::DuplicateEnumMember { enum_name, member }) => {
                assert_eq!(enum_name, "vfo");
                assert_eq!(member, "A");
            }
            other => panic!("Expected DuplicateEnumMember error, got {other:?}"),
        }
    }

    #[test]
    fn test_undefined_enum_type() {
        let schema_str = r#"
            [general]
            rig_type = "transceiver"
            version = 1

            [commands.set_freq]
            params = [["target", "undefined_enum"]]
        "#;

        let schema: Schema = toml::from_str(schema_str).unwrap();
        match schema.validate() {
            Err(SchemaError::UndefinedType {
                command,
                param,
                type_name,
            }) => {
                assert_eq!(command, "set_freq");
                assert_eq!(param, "target");
                assert_eq!(type_name, "undefined_enum");
            }
            other => panic!("Expected UndefinedType error, got {other:?}"),
        }
    }

    #[test]
    fn test_duplicate_param_name() {
        let schema_str = r#"
            [general]
            rig_type = "transceiver"
            version = 1

            [commands.set_freq]
            params = [
                ["freq", "int"],
                ["freq", "int"]
            ]
        "#;

        let schema: Schema = toml::from_str(schema_str).unwrap();
        match schema.validate() {
            Err(SchemaError::DuplicateParameter { command, param }) => {
                assert_eq!(command, "set_freq");
                assert_eq!(param, "freq");
            }
            other => panic!("Expected DuplicateParameter error, got {other:?}"),
        }
    }
}
