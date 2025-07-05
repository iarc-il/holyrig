use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::HashMap,
    error::Error,
    fmt::{Display, Write},
};

use crate::data_format::DataFormat;

#[derive(Debug)]
pub enum ParseError {
    InvalidMask,
    OddMaskLength,
    OddPlaceholdersLength,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for ParseError {}

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

// This is the "11.22.??.44" syntax that defines masks
#[derive(Debug, PartialEq, Eq)]
pub struct HexMask {
    pub data: Vec<u8>,
    // (index, length)
    pub masks: Vec<(usize, usize)>,
}

impl TryFrom<&str> for HexMask {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut result = HexMask {
            data: vec![],
            masks: vec![],
        };

        let mut current_byte: Option<u8> = None;
        let mut placeholder_count = 0;
        let mut index = 0;

        for c in value.chars().filter(|c| !c.is_whitespace() && *c != '.') {
            match c {
                '?' => {
                    if current_byte.is_some() {
                        return Err(ParseError::OddMaskLength);
                    }
                    placeholder_count += 1;
                    if placeholder_count == 2 {
                        result.data.push(0);
                        result.masks.push((index, 1));
                        index += 1;
                        placeholder_count = 0;
                    }
                }
                '0'..='9' | 'a'..='f' | 'A'..='F' => {
                    if placeholder_count > 0 {
                        return Err(ParseError::OddPlaceholdersLength);
                    }
                    let digit = c.to_digit(16).unwrap() as u8;
                    if let Some(high) = current_byte.take() {
                        result.data.push((high << 4) | digit);
                        index += 1;
                    } else {
                        current_byte = Some(digit);
                    }
                }
                _ => continue,
            }
        }

        if placeholder_count > 0 {
            return Err(ParseError::OddPlaceholdersLength);
        }

        if current_byte.is_some() {
            return Err(ParseError::OddMaskLength);
        }

        Ok(result)
    }
}

impl From<&HexMask> for String {
    fn from(value: &HexMask) -> Self {
        let mut result = String::new();
        let mut mask_iter = value.masks.iter().peekable();
        let mut current_mask = mask_iter.next();

        for (i, byte) in value.data.iter().enumerate() {
            if let Some(&(start, len)) = current_mask {
                if i == start {
                    result.push_str(&"?".repeat(len * 2));
                    current_mask = mask_iter.next();
                    continue;
                }
            }
            if i > 0 {
                result.push('.');
            }
            write!(result, "{byte:02X}").unwrap();
        }
        result
    }
}

fn serialize_hex_mask<S>(hex_mask: &HexMask, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let raw: String = hex_mask.into();
    serializer.serialize_str(raw.as_str())
}

fn deserialize_hex_mask<'de, D>(deserializer: D) -> Result<HexMask, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    HexMask::try_from(raw.as_str()).map_err(serde::de::Error::custom)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandFormat {
    #[serde(serialize_with = "serialize_hex_mask")]
    #[serde(deserialize_with = "deserialize_hex_mask")]
    pub command: HexMask,
    #[serde(flatten)]
    pub validator: Option<CommandValidator>,
    #[serde(default)]
    pub params: HashMap<String, CommandParam>,
}

fn default_multiply() -> u32 {
    1
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandParam {
    pub index: u32,
    pub length: u32,
    #[serde(deserialize_with = "deserialize_data_format")]
    pub format: DataFormat,
    #[serde(default)]
    pub add: i32,
    #[serde(default = "default_multiply")]
    pub multiply: u32,
}

fn deserialize_data_format<'de, D>(deserializer: D) -> Result<DataFormat, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    DataFormat::try_from(buf.as_str()).map_err(serde::de::Error::custom)
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_basic_mask() -> Result<(), ParseError> {
        let mask = HexMask::try_from("FEFE94E025??FD")?;
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

        let cmd: CommandFormat = toml::from_str(toml_str).unwrap();

        let freq_param = cmd.params.get("freq").unwrap();
        assert_eq!(freq_param.index, 6);
        assert_eq!(freq_param.length, 5);
        assert!(matches!(freq_param.format, DataFormat::BcdLu));
        assert_eq!(freq_param.add, 0);
        assert_eq!(freq_param.multiply, 1);

        let vfo_param = cmd.params.get("vfo").unwrap();
        assert_eq!(vfo_param.index, 5);
        assert_eq!(vfo_param.length, 1);
        assert!(matches!(vfo_param.format, DataFormat::IntLu));
        assert_eq!(vfo_param.add, 0);
        assert_eq!(vfo_param.multiply, 1);
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

        let cmd: CommandFormat = toml::from_str(toml_str).unwrap();

        let pitch_param = cmd.params.get("pitch").unwrap();
        assert_eq!(pitch_param.index, 6);
        assert_eq!(pitch_param.length, 2);
        assert!(matches!(pitch_param.format, DataFormat::BcdBu));
        assert_eq!(pitch_param.add, -127);
        assert_eq!(pitch_param.multiply, 4);
    }
}
