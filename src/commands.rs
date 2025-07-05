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
    UncoveredMask,
    OverlappingParams,
    MissingArgument(String),
    UnexpectedArgument(String),
    InvalidArgumentValue(String),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidMask => write!(f, "Invalid mask format"),
            ParseError::OddMaskLength => write!(f, "Mask length must be even"),
            ParseError::OddPlaceholdersLength => write!(f, "Placeholder length must be even"),
            ParseError::UncoveredMask => write!(f, "Mask region not covered by parameters"),
            ParseError::OverlappingParams => write!(f, "Parameters overlap"),
            ParseError::MissingArgument(param) => {
                write!(f, "Missing argument for parameter {param}")
            }
            ParseError::UnexpectedArgument(param) => {
                write!(f, "Unexpected argument for parameter {param}")
            }
            ParseError::InvalidArgumentValue(msg) => write!(f, "Invalid argument value: {msg}"),
        }
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

impl HexMask {
    pub fn validate_params(&self, params: &HashMap<String, BinaryParam>) -> Result<(), ParseError> {
        let mut param_regions: Vec<(usize, usize)> = params
            .values()
            .map(|param| (param.index as usize, param.length as usize))
            .collect();

        param_regions.sort_by_key(|&(start, _)| start);

        for i in 0..param_regions.len() - 1 {
            let (start1, len1) = param_regions[i];
            let (start2, _) = param_regions[i + 1];

            if start1 + len1 > start2 {
                return Err(ParseError::OverlappingParams);
            }
        }

        let mut covered_regions: Vec<(usize, usize)> = param_regions;
        covered_regions.sort_by_key(|&(start, _)| start);

        for &(mask_start, mask_len) in &self.masks {
            let mut is_covered = false;

            for &(param_start, param_len) in &covered_regions {
                if param_start <= mask_start && param_start + param_len >= mask_start + mask_len {
                    is_covered = true;
                    break;
                }
            }

            if !is_covered {
                return Err(ParseError::UncoveredMask);
            }
        }

        Ok(())
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

#[derive(Debug, Clone)]
pub enum BinaryParamArg {
    Int(i64),
    Bool(bool),
    Enum(String),
}

impl Command {
    pub fn build_command(
        &self,
        args: &HashMap<String, BinaryParamArg>,
    ) -> Result<Vec<u8>, ParseError> {
        // Validate parameters first
        self.command.validate_params(&self.params)?;

        // Validate arguments match parameters
        for param_name in self.params.keys() {
            if !args.contains_key(param_name) {
                return Err(ParseError::MissingArgument(param_name.clone()));
            }
        }
        for arg_name in args.keys() {
            if !self.params.contains_key(arg_name) {
                return Err(ParseError::UnexpectedArgument(arg_name.clone()));
            }
        }

        // Start with the base command data
        let mut result = self.command.data.clone();

        // Apply each argument to its parameter
        for (param_name, param) in &self.params {
            let arg = args.get(param_name).unwrap();
            let value = self.convert_arg_to_value(arg, param)?;
            self.apply_value_to_command(&mut result, value, param)?;
        }

        Ok(result)
    }

    fn convert_arg_to_value(
        &self,
        arg: &BinaryParamArg,
        param: &BinaryParam,
    ) -> Result<i64, ParseError> {
        let raw_value = match arg {
            BinaryParamArg::Int(v) => *v,
            BinaryParamArg::Bool(v) => {
                if *v {
                    1
                } else {
                    0
                }
            }
            BinaryParamArg::Enum(_) => todo!("Enum handling not implemented yet"),
        };

        let value = (raw_value + param.add as i64) * param.multiply as i64;

        Ok(value)
    }

    fn apply_value_to_command(
        &self,
        data: &mut [u8],
        value: i64,
        param: &BinaryParam,
    ) -> Result<(), ParseError> {
        let start = param.index as usize;
        let len = param.length as usize;

        if start + len > data.len() {
            return Err(ParseError::InvalidArgumentValue(
                "Parameter position exceeds command length".to_string(),
            ));
        }

        match param.format {
            DataFormat::BcdBu => {
                if value < 0 {
                    return Err(ParseError::InvalidArgumentValue(
                        "Negative value not allowed for unsigned BCD".to_string(),
                    ));
                }
                let bcd = format!("{:0width$}", value, width = len * 2);
                if bcd.len() != len * 2 {
                    return Err(ParseError::InvalidArgumentValue(
                        "Value too large for BCD format".to_string(),
                    ));
                }
                for (i, chunk) in bcd.as_bytes().chunks(2).enumerate() {
                    let byte = ((chunk[0] - b'0') << 4) | (chunk[1] - b'0');
                    data[start + i] = byte;
                }
            }
            _ => todo!("Other formats not implemented yet"),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_params() {
        let mask = HexMask::try_from("1122??44??66").unwrap();
        let mut params = HashMap::new();
        params.insert(
            "param1".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0,
                multiply: 1,
            },
        );
        params.insert(
            "param2".to_string(),
            BinaryParam {
                index: 4,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0,
                multiply: 1,
            },
        );
        assert!(mask.validate_params(&params).is_ok());
    }

    #[test]
    fn test_valid_subsequent_params() {
        let mask = HexMask::try_from("11????????66").unwrap();
        let mut params = HashMap::new();
        params.insert(
            "param1".to_string(),
            BinaryParam {
                index: 1,
                length: 2,
                format: DataFormat::BcdBu,
                add: 0,
                multiply: 1,
            },
        );
        params.insert(
            "param2".to_string(),
            BinaryParam {
                index: 3,
                length: 2,
                format: DataFormat::BcdBu,
                add: 0,
                multiply: 1,
            },
        );
        assert!(mask.validate_params(&params).is_ok());
    }

    #[test]
    fn test_overlapping_params() {
        let mask = HexMask::try_from("11????44").unwrap();
        let mut params = HashMap::new();
        params.insert(
            "param1".to_string(),
            BinaryParam {
                index: 1,
                length: 2,
                format: DataFormat::BcdBu,
                add: 0,
                multiply: 1,
            },
        );
        params.insert(
            "param2".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0,
                multiply: 1,
            },
        );
        assert!(matches!(
            mask.validate_params(&params),
            Err(ParseError::OverlappingParams)
        ));
    }

    #[test]
    fn test_uncovered_mask() {
        let mask = HexMask::try_from("11????44??").unwrap();
        let mut params = HashMap::new();
        params.insert(
            "param1".to_string(),
            BinaryParam {
                index: 1,
                length: 2,
                format: DataFormat::BcdBu,
                add: 0,
                multiply: 1,
            },
        );
        assert!(matches!(
            mask.validate_params(&params),
            Err(ParseError::UncoveredMask)
        ));
    }

    #[test]
    fn test_gap_between_params() {
        let mask = HexMask::try_from("11????????66").unwrap();
        let mut params = HashMap::new();
        params.insert(
            "param1".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0,
                multiply: 1,
            },
        );
        params.insert(
            "param2".to_string(),
            BinaryParam {
                index: 4,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0,
                multiply: 1,
            },
        );
        assert!(matches!(
            mask.validate_params(&params),
            Err(ParseError::UncoveredMask)
        ));
    }

    #[test]
    fn test_build_command_valid() {
        let mask = HexMask::try_from("1122??44").unwrap();
        let mut params = HashMap::new();
        params.insert(
            "freq".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0,
                multiply: 1,
            },
        );

        let cmd = Command {
            command: mask,
            validator: None,
            params,
        };

        let mut args = HashMap::new();
        args.insert("freq".to_string(), BinaryParamArg::Int(42));

        let result = cmd.build_command(&args).unwrap();
        assert_eq!(result, vec![0x11, 0x22, 0x42, 0x44]);
    }

    #[test]
    fn test_build_command_missing_arg() {
        let mask = HexMask::try_from("1122??44").unwrap();
        let mut params = HashMap::new();
        params.insert(
            "freq".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0,
                multiply: 1,
            },
        );

        let cmd = Command {
            command: mask,
            validator: None,
            params,
        };

        let args = HashMap::new();
        assert!(matches!(
            cmd.build_command(&args),
            Err(ParseError::MissingArgument(_))
        ));
    }

    #[test]
    fn test_build_command_unexpected_arg() {
        let mask = HexMask::try_from("1122??44").unwrap();
        let mut params = HashMap::new();
        params.insert(
            "freq".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0,
                multiply: 1,
            },
        );

        let cmd = Command {
            command: mask,
            validator: None,
            params,
        };

        let mut args = HashMap::new();
        args.insert("freq".to_string(), BinaryParamArg::Int(42));
        args.insert("unknown".to_string(), BinaryParamArg::Int(10));

        assert!(matches!(
            cmd.build_command(&args),
            Err(ParseError::UnexpectedArgument(_))
        ));
    }

    #[test]
    fn test_build_command_with_transforms() {
        let mask = HexMask::try_from("1122??44").unwrap();
        let mut params = HashMap::new();
        params.insert(
            "freq".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 10,
                multiply: 2,
            },
        );

        let cmd = Command {
            command: mask,
            validator: None,
            params,
        };

        let mut args = HashMap::new();
        args.insert("freq".to_string(), BinaryParamArg::Int(11));

        let result = cmd.build_command(&args).unwrap();
        assert_eq!(result, vec![0x11, 0x22, 0x42, 0x44]);
    }

    #[test]
    fn test_build_command_invalid_bcd() {
        let mask = HexMask::try_from("1122??44").unwrap();
        let mut params = HashMap::new();
        params.insert(
            "freq".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0,
                multiply: 1,
            },
        );

        let cmd = Command {
            command: mask,
            validator: None,
            params,
        };

        let mut args = HashMap::new();
        args.insert("freq".to_string(), BinaryParamArg::Int(-1));

        assert!(matches!(
            cmd.build_command(&args),
            Err(ParseError::InvalidArgumentValue(_))
        ));

        args.insert("freq".to_string(), BinaryParamArg::Int(100));
        assert!(matches!(
            cmd.build_command(&args),
            Err(ParseError::InvalidArgumentValue(_))
        ));
    }
}
