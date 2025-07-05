use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::HashMap,
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
pub struct Command {
    #[serde(serialize_with = "serialize_hex_mask")]
    #[serde(deserialize_with = "deserialize_hex_mask")]
    pub command: HexMask,
    #[serde(flatten)]
    pub validator: Option<CommandValidator>,
    #[serde(default)]
    pub params: HashMap<String, BinaryParam>,
}

fn default_multiply() -> u32 {
    1
}

// The binary param struct is used to build commands from given argument and parse data from
// responses.
#[derive(Debug, Serialize, Deserialize)]
pub struct BinaryParam {
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
