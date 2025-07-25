use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::path::Path;

use crate::commands::Value;

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
    DuplicateReturn {
        command: String,
        return_name: String,
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
            SchemaError::DuplicateReturn {
                command,
                return_name,
            } => {
                write!(
                    f,
                    "Duplicate return value {return_name} in command {command}"
                )
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

#[derive(Debug, Clone, Deserialize)]
pub struct General {
    pub rig_type: String,
    pub version: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Enum {
    pub members: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ValueType {
    Int,
    Bool,
    Enum(String),
}

impl ValueType {
    pub fn build_value(&self, value: &str) -> anyhow::Result<Value> {
        let result = match self {
            ValueType::Int => Value::Int(value.parse()?),
            ValueType::Bool => Value::Bool(value.parse()?),
            ValueType::Enum(_) => Value::Enum(value.to_string()),
        };
        Ok(result)
    }
}

impl<'de> Deserialize<'de> for ValueType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw_value = String::deserialize(deserializer)?;
        match raw_value.as_str() {
            "int" => Ok(ValueType::Int),
            "bool" => Ok(ValueType::Bool),
            other => Ok(ValueType::Enum(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Command {
    #[serde(default)]
    pub params: Vec<(String, ValueType)>,
    #[serde(default)]
    pub returns: Vec<(String, ValueType)>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Schema {
    pub general: General,
    #[serde(default)]
    pub enums: HashMap<String, Enum>,
    pub commands: HashMap<String, Command>,
}

#[cfg(test)]
impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

impl Schema {
    // Add new() method for tests
    #[cfg(test)]
    pub fn new() -> Self {
        Self {
            general: General {
                rig_type: "transceiver".to_string(),
                version: 1,
            },
            enums: HashMap::new(),
            commands: HashMap::new(),
        }
    }

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
            let mut seen_params = HashSet::new();
            for (param_name, type_name) in &cmd.params {
                if !seen_params.insert(param_name) {
                    return Err(SchemaError::DuplicateParameter {
                        command: cmd_name.clone(),
                        param: param_name.clone(),
                    });
                }
                if let ValueType::Enum(type_name) = &type_name
                    && !self.enums.contains_key(type_name)
                {
                    return Err(SchemaError::UndefinedType {
                        command: cmd_name.clone(),
                        param: param_name.clone(),
                        type_name: type_name.clone(),
                    });
                }
            }

            let mut seen_returns = HashSet::new();
            for (return_name, type_name) in &cmd.returns {
                if !seen_returns.insert(return_name) {
                    return Err(SchemaError::DuplicateReturn {
                        command: cmd_name.clone(),
                        return_name: return_name.clone(),
                    });
                }
                if let ValueType::Enum(type_name) = &type_name
                    && !self.enums.contains_key(type_name)
                {
                    return Err(SchemaError::UndefinedType {
                        command: cmd_name.clone(),
                        param: return_name.clone(),
                        type_name: type_name.clone(),
                    });
                }
            }
        }

        Ok(())
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

    #[test]
    fn test_schema_with_valid_enums() {
        let mut schema = Schema::new();

        let mode_enum = Enum {
            members: vec!["LSB".to_string(), "USB".to_string(), "CW".to_string()],
        };
        schema.enums.insert("Mode".to_string(), mode_enum);

        let cmd = Command {
            params: vec![("mode".to_string(), ValueType::Enum("Mode".to_string()))],
            returns: vec![],
        };
        schema.commands.insert("set_mode".to_string(), cmd);

        assert!(schema.validate().is_ok());
    }

    #[test]
    fn test_schema_with_invalid_enum_reference() {
        let mut schema = Schema::new();

        let cmd = Command {
            params: vec![(
                "mode".to_string(),
                ValueType::Enum("NonExistentEnum".to_string()),
            )],
            returns: vec![],
        };
        schema.commands.insert("set_mode".to_string(), cmd);

        assert!(schema.validate().is_err());
    }

    #[test]
    fn test_schema_with_empty_enum() {
        let mut schema = Schema::new();

        let empty_enum = Enum { members: vec![] };
        schema.enums.insert("EmptyEnum".to_string(), empty_enum);

        let cmd = Command {
            params: vec![(
                "param".to_string(),
                ValueType::Enum("EmptyEnum".to_string()),
            )],
            returns: vec![],
        };
        schema.commands.insert("test_cmd".to_string(), cmd);

        assert!(schema.validate().is_err());
    }

    #[test]
    fn test_duplicate_return_value() {
        let mut schema = Schema::new();

        let cmd = Command {
            params: vec![("param".to_string(), ValueType::Int)],
            returns: vec![
                ("result".to_string(), ValueType::Int),
                ("result".to_string(), ValueType::Int),
            ],
        };
        schema.commands.insert("test_cmd".to_string(), cmd);

        assert!(schema.validate().is_err());
    }
}
