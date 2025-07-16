use crate::commands::CommandError;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

use crate::commands::{BinMask, BinaryParam, Command, CommandValidator};
use crate::data_format::DataFormat;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct General {
    pub r#type: String,
    pub version: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RigCommand {
    pub command: String,
    pub response: Option<String>,
    pub reply_length: Option<usize>,
    pub reply_end: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, RigBinaryParam>,
    #[serde(default)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub returns: HashMap<String, RigBinaryParam>,
}

fn deserialize_data_format<'de, D>(deserializer: D) -> Result<DataFormat, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    DataFormat::try_from(buf.as_str()).map_err(serde::de::Error::custom)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RigBinaryParam {
    pub index: u32,
    pub length: u32,
    #[serde(deserialize_with = "deserialize_data_format")]
    pub format: DataFormat,
    #[serde(default)]
    pub add: f64,
    #[serde(default = "default_multiply")]
    pub multiply: f64,
}

fn default_multiply() -> f64 {
    1.0
}

impl TryFrom<RigCommand> for Command {
    type Error = CommandError;

    fn try_from(value: RigCommand) -> Result<Self, Self::Error> {
        let command = BinMask::try_from(value.command.as_str())?;

        let validator = match (value.reply_length, value.reply_end) {
            (Some(length), None) => Some(CommandValidator::ReplyLength(length)),
            (None, Some(end)) => Some(CommandValidator::ReplyEnd(end)),
            (None, None) => None,
            _ => {
                return Err(CommandError::MultipleValidators);
            }
        };

        let response = if let Some(response) = value.response {
            Some(BinMask::try_from(response.as_str())?)
        } else {
            None
        };

        let mut params = HashMap::new();
        for (name, param) in value.params {
            params.insert(
                name,
                BinaryParam {
                    index: param.index,
                    length: param.length,
                    format: param.format,
                    add: param.add,
                    multiply: param.multiply,
                },
            );
        }

        let mut returns = HashMap::new();
        for (name, param) in value.returns {
            returns.insert(
                name,
                BinaryParam {
                    index: param.index,
                    length: param.length,
                    format: param.format,
                    add: param.add,
                    multiply: param.multiply,
                },
            );
        }

        let result = Command {
            command,
            response,
            validator,
            params,
            returns,
        };
        result.validate()?;
        Ok(result)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RigFile {
    pub general: General,
    #[serde(default)]
    pub init: Vec<RigCommand>,
    pub commands: HashMap<String, RigCommand>,
    #[serde(default)]
    pub status: Vec<RigCommand>,
}

impl RigFile {
    pub fn new() -> Self {
        Self {
            general: General {
                r#type: "transceiver".to_string(),
                version: 1,
            },
            init: Vec::new(),
            commands: HashMap::new(),
            status: Vec::new(),
        }
    }
}

impl Default for RigFile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{BinMask, CommandError};
    use crate::data_format::DataFormat;
    use anyhow::Result;

    #[test]
    fn test_basic_mask() -> Result<(), CommandError> {
        let mask = BinMask::try_from("FEFE94E025??FD")?;
        assert_eq!(mask.data, vec![0xFE, 0xFE, 0x94, 0xE0, 0x25, 0x00, 0xFD]);
        assert_eq!(mask.masks, vec![(5, 1)]);
        Ok(())
    }

    #[test]
    fn test_command_params() {
        let toml_str = r#"
            command = 'FEFE94E0.25.??.??.FD'

            [params.freq]
            index = 6
            length = 5
            format = "bcd_lu"

            [params.vfo]
            index = 5
            length = 1
            format = "int_lu"
        "#;

        let cmd: RigCommand = toml::from_str(toml_str).unwrap();

        let freq_param = cmd.params.get("freq").unwrap();
        assert_eq!(freq_param.index, 6);
        assert_eq!(freq_param.length, 5);
        assert!(matches!(freq_param.format, DataFormat::BcdLu));
        assert_eq!(freq_param.add, 0.0);
        assert_eq!(freq_param.multiply, 1.0);

        let vfo_param = cmd.params.get("vfo").unwrap();
        assert_eq!(vfo_param.index, 5);
        assert_eq!(vfo_param.length, 1);
        assert!(matches!(vfo_param.format, DataFormat::IntLu));
        assert_eq!(vfo_param.add, 0.0);
        assert_eq!(vfo_param.multiply, 1.0);
    }

    #[test]
    fn test_command_params_with_add_multiply() {
        let toml_str = r#"
            command = 'FEFE94E0.14.09.00.00'

            [params.pitch]
            index = 6
            length = 2
            format = "bcd_bu"
            add = -127
            multiply = 4
        "#;

        let cmd: RigCommand = toml::from_str(toml_str).unwrap();

        let pitch_param = cmd.params.get("pitch").unwrap();
        assert_eq!(pitch_param.index, 6);
        assert_eq!(pitch_param.length, 2);
        assert!(matches!(pitch_param.format, DataFormat::BcdBu));
        assert_eq!(pitch_param.add, -127.0);
        assert_eq!(pitch_param.multiply, 4.0);
    }

    #[test]
    fn test_command_returns() {
        let toml_str = r#"
            command = 'FEFE94E0.25.00.FD'
            response = 'FEFE94E0.25.??.??.FD'

            [returns.freq]
            index = 5
            length = 2
            format = "bcd_lu"
            add = 0
            multiply = 1
        "#;

        let cmd: RigCommand = toml::from_str(toml_str).unwrap();

        let freq_return = cmd.returns.get("freq").unwrap();
        assert_eq!(freq_return.index, 5);
        assert_eq!(freq_return.length, 2);
        assert!(matches!(freq_return.format, DataFormat::BcdLu));
        assert_eq!(freq_return.add, 0.0);
        assert_eq!(freq_return.multiply, 1.0);
    }

    #[test]
    fn test_command_returns_with_add_multiply() {
        let toml_str = r#"
            command = 'FEFE94E0.14.09.00.00'
            response = 'FEFE94E0.14.??.??.00'

            [returns.pitch]
            index = 5
            length = 2
            format = "bcd_bu"
            add = -127
            multiply = 4
        "#;

        let cmd: RigCommand = toml::from_str(toml_str).unwrap();

        let pitch_return = cmd.returns.get("pitch").unwrap();
        assert_eq!(pitch_return.index, 5);
        assert_eq!(pitch_return.length, 2);
        assert!(matches!(pitch_return.format, DataFormat::BcdBu));
        assert_eq!(pitch_return.add, -127.0);
        assert_eq!(pitch_return.multiply, 4.0);
    }

    #[test]
    fn test_command_returns_conversion() -> Result<(), CommandError> {
        let toml_str = r#"
            command = 'FEFE94E0.25.00.FD'
            response = 'FEFE94E0.25.??.??.FD'

            [returns.freq]
            index = 5
            length = 2
            format = "bcd_lu"
            add = 0
            multiply = 1
        "#;

        let rig_cmd: RigCommand = toml::from_str(toml_str).unwrap();
        let cmd = Command::try_from(rig_cmd)?;

        let freq_return = cmd.returns.get("freq").unwrap();
        assert_eq!(freq_return.index, 5);
        assert_eq!(freq_return.length, 2);
        assert!(matches!(freq_return.format, DataFormat::BcdLu));
        assert_eq!(freq_return.add, 0.0);
        assert_eq!(freq_return.multiply, 1.0);

        Ok(())
    }

    #[test]
    fn test_command_returns_without_response() {
        let toml_str = r#"
            command = 'FEFE94E0.25.00.FD'

            [returns.freq]
            index = 5
            length = 2
            format = "bcd_lu"
        "#;

        let rig_cmd: RigCommand = toml::from_str(toml_str).unwrap();
        println!("Rig cmd: {rig_cmd:?}");
        let result = Command::try_from(rig_cmd);

        assert!(matches!(
            result,
            Err(CommandError::ReturnValuesWithoutResponse)
        ));
    }
}
