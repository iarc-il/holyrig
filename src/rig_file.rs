use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{collections::HashMap, error::Error, fmt::Display};

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
    data: Vec<u8>,
    // (index, length)
    masks: Vec<(usize, usize)>,
}

impl TryFrom<&str> for HexMask {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut result = HexMask {
            data: vec![],
            masks: vec![],
        };
        let mut current_byte = None;
        let mut current_mask = None;
        let mut index = 0;
        for byte in value.bytes() {
            match byte as char {
                '.' => {
                    continue;
                }
                '?' => {
                    if let Some((index, length)) = current_mask {
                        current_mask = Some((index, length + 1));
                    } else {
                        current_mask = Some((index, 1));
                    }
                }
                current_char => {
                    if let Some((mask_index, length)) = std::mem::take(&mut current_mask) {
                        if length % 2 != 0 {
                            return Err(ParseError::OddPlaceholdersLength);
                        }
                        let length = length / 2;
                        index += length;
                        result.data.extend(std::iter::repeat_n(0, length));
                        result.masks.push((mask_index, length));
                    }
                    let base_value = match current_char {
                        '0'..='9' => '0',
                        'A'..='F' => 'A',
                        _ => {
                            return Err(ParseError::InvalidMask);
                        }
                    };

                    let value = byte - (base_value as u8);
                    if let Some(byte) = current_byte {
                        result.data.push((byte << 4) + value);
                        index += 1;
                        current_byte = None;
                    } else {
                        current_byte = Some(value);
                    }
                }
            }
        }

        if let Some((index, length)) = std::mem::take(&mut current_mask) {
            if length % 2 != 0 {
                return Err(ParseError::OddPlaceholdersLength);
            }
            result.data.extend(std::iter::repeat_n(0, length / 2));
            result.masks.push((index, length / 2));
        }

        if current_byte.is_some() {
            return Err(ParseError::OddMaskLength);
        }

        Ok(result)
    }
}

impl From<&HexMask> for String {
    fn from(_value: &HexMask) -> Self {
        todo!()
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
    let raw: &str = Deserialize::deserialize(deserializer)?;
    Ok(HexMask::try_from(raw).unwrap())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandFormat {
    #[serde(serialize_with = "serialize_hex_mask")]
    #[serde(deserialize_with = "deserialize_hex_mask")]
    pub command: HexMask,
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
    #[serde(default = "default_multiply")]
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_basic_mask() -> Result<()> {
        let mask = "11.22.??.44";
        let result = HexMask::try_from(mask)?;
        let expected = HexMask {
            data: vec![0x11, 0x22, 0, 0x44],
            masks: vec![(2, 1)],
        };
        assert!(result == expected);
        Ok(())
    }
}
