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

#[derive(Debug)]
pub enum CommandValidator {
    ReplyLength(u32),
    ReplyEnd(String),
    Mask(HexMask),
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

#[derive(Debug)]
pub struct Command {
    pub command: HexMask,
    pub validator: Option<CommandValidator>,
    pub params: HashMap<String, BinaryParam>,
}

// The binary param struct is used to build commands from given argument and parse data from
// responses.
#[derive(Debug)]
pub struct BinaryParam {
    pub index: u32,
    pub length: u32,
    pub format: DataFormat,
    pub add: i32,
    pub multiply: u32,
}
