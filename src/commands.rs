use std::{
    collections::BTreeMap,
    error::Error,
    fmt::{Display, Write},
};

use crate::data_format::{DataFormat, DataFormatError};

#[derive(Debug)]
pub enum CommandError {
    InvalidMask,
    OddMaskLength,
    OddPlaceholdersLength,
    UncoveredMask,
    UncoveredParam,
    OverlappingParams,
    DataFormat(DataFormatError),
    MissingArgument(String),
    UnexpectedArgument(String),
    InvalidArgumentValue(String),
    MultipleValidators,
    ReturnValuesWithoutResponse,
}

impl Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::InvalidMask => write!(f, "Invalid mask format"),
            CommandError::OddMaskLength => write!(f, "Mask length must be even"),
            CommandError::OddPlaceholdersLength => write!(f, "Placeholder length must be even"),
            CommandError::UncoveredMask => write!(f, "Mask region not covered by parameters"),
            CommandError::UncoveredParam => write!(f, "Parameter is not covered by mask region"),
            CommandError::OverlappingParams => write!(f, "Parameters overlap"),
            CommandError::DataFormat(data_format_error) => {
                write!(f, "Data format error: {data_format_error}")
            }
            CommandError::MissingArgument(param) => {
                write!(f, "Missing argument for parameter {param}")
            }
            CommandError::UnexpectedArgument(param) => {
                write!(f, "Unexpected argument for parameter {param}")
            }
            CommandError::InvalidArgumentValue(msg) => write!(f, "Invalid argument value: {msg}"),
            CommandError::ReturnValuesWithoutResponse => {
                write!(f, "Return values without response")
            }
            CommandError::MultipleValidators => write!(f, "Multiple validators"),
        }
    }
}
impl Error for CommandError {}

impl From<DataFormatError> for CommandError {
    fn from(value: DataFormatError) -> Self {
        CommandError::DataFormat(value)
    }
}

#[derive(Debug, Clone)]
pub enum CommandValidator {
    ReplyLength(usize),
    ReplyEnd(String),
}

// This is the "11.22.??.44" syntax that defines masks
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinMask {
    pub data: Vec<u8>,
    // (index, length)
    pub masks: Vec<(usize, usize)>,
}

impl TryFrom<&str> for BinMask {
    type Error = CommandError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut result = BinMask {
            data: vec![],
            masks: vec![],
        };

        let chunks = value
            .chars()
            .filter_map(|c| {
                if c == '.' {
                    None
                } else if c.is_ascii_hexdigit() || c == '?' {
                    Some(Ok(c))
                } else {
                    Some(Err(CommandError::InvalidMask))
                }
            })
            .collect::<Result<Vec<_>, CommandError>>()?
            .chunk_by(|c1, c2| c1.is_ascii_hexdigit() == c2.is_ascii_hexdigit())
            .map(Vec::from)
            .collect::<Vec<_>>();

        let mut current_index = 0;
        for chunk in chunks {
            let length = chunk.len() / 2;
            if chunk[0].is_ascii_hexdigit() {
                if !chunk.len().is_multiple_of(2) {
                    return Err(CommandError::OddMaskLength);
                }
                let data = chunk.chunks_exact(2).map(|pair| {
                    let &[c1, c2] = pair else {
                        panic!();
                    };
                    let high_digit = c1.to_digit(16).unwrap() as u8;
                    let low_digit = c2.to_digit(16).unwrap() as u8;
                    (high_digit << 4) | low_digit
                });
                result.data.extend(data);
            } else {
                if !chunk.len().is_multiple_of(2) {
                    return Err(CommandError::OddPlaceholdersLength);
                }
                result.data.extend([0].repeat(length));
                result.masks.push((current_index, length));
            }
            current_index += length;
        }

        Ok(result)
    }
}

impl From<&BinMask> for String {
    fn from(value: &BinMask) -> Self {
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

impl BinMask {
    pub fn validate_params(
        &self,
        params: &BTreeMap<String, BinaryParam>,
    ) -> Result<(), CommandError> {
        if params.is_empty() {
            return Ok(());
        }

        let mut param_regions: Vec<(usize, usize)> = params
            .values()
            .map(|param| (param.index as usize, param.length as usize))
            .collect();

        param_regions.sort_by_key(|&(start, _)| start);

        for i in 0..param_regions.len() - 1 {
            let (start1, len1) = param_regions[i];
            let (start2, _) = param_regions[i + 1];

            if start1 + len1 > start2 {
                return Err(CommandError::OverlappingParams);
            }
        }

        for &(param_start, param_len) in &param_regions {
            let mut is_covered = false;

            for &(mask_start, mask_len) in &self.masks {
                if mask_start <= param_start && mask_start + mask_len >= param_start + param_len {
                    is_covered = true;
                    break;
                }
            }

            if !is_covered {
                return Err(CommandError::UncoveredParam);
            }
        }

        for &(mask_start, mask_len) in &self.masks {
            let mut covered_regions = vec![false; mask_len];

            for &(param_start, param_len) in &param_regions {
                if param_start + param_len <= mask_start || param_start >= mask_start + mask_len {
                    continue;
                }

                let overlap_start = param_start.max(mask_start) - mask_start;
                let overlap_end = (param_start + param_len).min(mask_start + mask_len) - mask_start;

                (overlap_start..overlap_end).for_each(|i| {
                    covered_regions[i] = true;
                });
            }

            if covered_regions.iter().any(|&covered| !covered) {
                return Err(CommandError::UncoveredMask);
            }
        }

        Ok(())
    }

    pub fn validate_data(&self, data: &[u8]) -> Result<(), CommandError> {
        if data.len() != self.data.len() {
            return Err(CommandError::InvalidMask);
        }
        Ok(())
    }

    pub fn extract_value<'a>(
        &self,
        data: &'a [u8],
        start: usize,
        length: usize,
    ) -> Result<&'a [u8], CommandError> {
        if start + length > data.len() {
            return Err(CommandError::InvalidMask);
        }
        Ok(&data[start..start + length])
    }
}

#[derive(Debug, Clone)]
pub struct Command {
    pub command: BinMask,
    pub response: Option<BinMask>,
    pub validator: Option<CommandValidator>,
    pub params: BTreeMap<String, BinaryParam>,
    pub returns: BTreeMap<String, BinaryParam>,
}

// The binary param struct is used to build commands from given argument and parse data from
// responses.
#[derive(Debug, Clone)]
pub struct BinaryParam {
    pub index: u32,
    pub length: u32,
    pub format: DataFormat,
    pub add: f64,
    pub multiply: f64,
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Enum(String),
}

impl Command {
    pub fn validate(&self) -> Result<(), CommandError> {
        self.command.validate_params(&self.params)?;

        if let Some(response_mask) = &self.response {
            response_mask.validate_params(&self.returns)?;
        }

        if self.response.is_none() && !self.returns.is_empty() {
            return Err(CommandError::ReturnValuesWithoutResponse);
        }

        Ok(())
    }

    pub fn parse_response(&self, response: &[u8]) -> Result<BTreeMap<String, i64>, CommandError> {
        let mut result = BTreeMap::new();

        let response_mask = match &self.response {
            Some(mask) => mask,
            None => return Ok(result),
        };

        response_mask.validate_data(response)?;

        for (key, param) in &self.returns {
            let start = param.index as usize;
            let length = param.length as usize;

            let bytes = response_mask.extract_value(response, start, length)?;

            let raw_value = param.format.decode(bytes).map_err(|err| {
                CommandError::InvalidArgumentValue(format!("Failed to decode value: {err}"))
            })?;

            let transformed_value =
                ((raw_value as f64 - param.add) / param.multiply).round() as i64;

            result.insert(key.clone(), transformed_value);
        }

        Ok(result)
    }

    pub fn build_command(&self, args: &BTreeMap<String, Value>) -> Result<Vec<u8>, CommandError> {
        self.validate()?;

        for param_name in self.params.keys() {
            if !args.contains_key(param_name) {
                return Err(CommandError::MissingArgument(param_name.clone()));
            }
        }
        for arg_name in args.keys() {
            if !self.params.contains_key(arg_name) {
                return Err(CommandError::UnexpectedArgument(arg_name.clone()));
            }
        }

        let mut result = self.command.data.clone();

        for (param_name, param) in &self.params {
            let arg = args.get(param_name).unwrap();
            let value = self.convert_arg_to_value(arg, param)?;
            self.apply_value_to_command(&mut result, value, param)?;
        }

        Ok(result)
    }

    fn convert_arg_to_value(&self, arg: &Value, param: &BinaryParam) -> Result<i64, CommandError> {
        let raw_value = match arg {
            Value::Int(v) => *v as f64,
            Value::Bool(v) => {
                if *v {
                    1.0
                } else {
                    0.0
                }
            }
            Value::Enum(v) => {
                return Err(CommandError::InvalidArgumentValue(format!(
                    "Enum value '{v}' must be converted to integer by RigApi"
                )));
            }
        };

        let value = ((raw_value + param.add) * param.multiply).round() as i64;

        Ok(value)
    }

    fn apply_value_to_command(
        &self,
        data: &mut [u8],
        value: i64,
        param: &BinaryParam,
    ) -> Result<(), CommandError> {
        let start = param.index as usize;
        let len = param.length as usize;

        if start + len > data.len() {
            return Err(CommandError::InvalidArgumentValue(
                "Parameter position exceeds command length".to_string(),
            ));
        }

        let bytes = param.format.encode(value as i32, len)?;
        data[start..start + bytes.len()].copy_from_slice(&bytes);

        Ok(())
    }

    pub fn response_length(&self) -> Option<usize> {
        self.response
            .as_ref()
            .map(|mask| mask.data.len())
            .or(self
                .validator
                .as_ref()
                .and_then(|validator| match validator {
                    CommandValidator::ReplyLength(length) => Some(*length),
                    CommandValidator::ReplyEnd(_) => None,
                }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_format::DataFormat;

    #[test]
    fn test_valid_params() {
        let mask = BinMask::try_from("1122??44??66").unwrap();
        let mut params = BTreeMap::new();
        params.insert(
            "param1".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0.0,
                multiply: 1.0,
            },
        );
        params.insert(
            "param2".to_string(),
            BinaryParam {
                index: 4,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0.0,
                multiply: 1.0,
            },
        );
        assert!(mask.validate_params(&params).is_ok());
    }

    #[test]
    fn test_valid_subsequent_params() {
        let mask = BinMask::try_from("11????????66").unwrap();
        let mut params = BTreeMap::new();
        params.insert(
            "param1".to_string(),
            BinaryParam {
                index: 1,
                length: 2,
                format: DataFormat::BcdBu,
                add: 0.0,
                multiply: 1.0,
            },
        );
        params.insert(
            "param2".to_string(),
            BinaryParam {
                index: 3,
                length: 2,
                format: DataFormat::BcdBu,
                add: 0.0,
                multiply: 1.0,
            },
        );
        assert!(mask.validate_params(&params).is_ok());
    }

    #[test]
    fn test_overlapping_params() {
        let mask = BinMask::try_from("11????44").unwrap();
        let mut params = BTreeMap::new();
        params.insert(
            "param1".to_string(),
            BinaryParam {
                index: 1,
                length: 2,
                format: DataFormat::BcdBu,
                add: 0.0,
                multiply: 1.0,
            },
        );
        params.insert(
            "param2".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0.0,
                multiply: 1.0,
            },
        );
        assert!(matches!(
            mask.validate_params(&params),
            Err(CommandError::OverlappingParams)
        ));
    }

    #[test]
    fn test_uncovered_mask() {
        let mask = BinMask::try_from("11????44??").unwrap();
        let mut params = BTreeMap::new();
        params.insert(
            "param1".to_string(),
            BinaryParam {
                index: 1,
                length: 2,
                format: DataFormat::BcdBu,
                add: 0.0,
                multiply: 1.0,
            },
        );
        assert!(matches!(
            mask.validate_params(&params),
            Err(CommandError::UncoveredMask)
        ));
    }

    #[test]
    fn test_gap_between_params() {
        let mask = BinMask::try_from("11????????66").unwrap();
        let mut params = BTreeMap::new();
        params.insert(
            "param1".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0.0,
                multiply: 1.0,
            },
        );
        params.insert(
            "param2".to_string(),
            BinaryParam {
                index: 4,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0.0,
                multiply: 1.0,
            },
        );
        assert!(matches!(
            mask.validate_params(&params),
            Err(CommandError::UncoveredMask)
        ));
    }

    #[test]
    fn test_build_command_valid() {
        let mask = BinMask::try_from("1122??44").unwrap();
        let mut params = BTreeMap::new();
        params.insert(
            "freq".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0.0,
                multiply: 1.0,
            },
        );

        let cmd = Command {
            command: mask,
            response: None,
            validator: None,
            params,
            returns: BTreeMap::new(),
        };

        let mut args = BTreeMap::new();
        args.insert("freq".to_string(), Value::Int(42));

        let result = cmd.build_command(&args).unwrap();
        assert_eq!(result, vec![0x11, 0x22, 0x42, 0x44]);
    }

    #[test]
    fn test_build_command_missing_arg() {
        let mask = BinMask::try_from("1122??44").unwrap();
        let mut params = BTreeMap::new();
        params.insert(
            "freq".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0.0,
                multiply: 1.0,
            },
        );

        let cmd = Command {
            command: mask,
            response: None,
            validator: None,
            params,
            returns: BTreeMap::new(),
        };

        let args = BTreeMap::new();
        assert!(matches!(
            cmd.build_command(&args),
            Err(CommandError::MissingArgument(_))
        ));
    }

    #[test]
    fn test_build_command_unexpected_arg() {
        let mask = BinMask::try_from("1122??44").unwrap();
        let mut params = BTreeMap::new();
        params.insert(
            "freq".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0.0,
                multiply: 1.0,
            },
        );

        let cmd = Command {
            command: mask,
            response: None,
            validator: None,
            params,
            returns: BTreeMap::new(),
        };

        let mut args = BTreeMap::new();
        args.insert("freq".to_string(), Value::Int(42));
        args.insert("unknown".to_string(), Value::Int(10));

        assert!(matches!(
            cmd.build_command(&args),
            Err(CommandError::UnexpectedArgument(_))
        ));
    }

    #[test]
    fn test_build_command_with_transforms() {
        let mask = BinMask::try_from("1122??44").unwrap();
        let mut params = BTreeMap::new();
        params.insert(
            "freq".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 10.0,
                multiply: 2.0,
            },
        );

        let cmd = Command {
            command: mask,
            response: None,
            validator: None,
            params,
            returns: BTreeMap::new(),
        };

        let mut args = BTreeMap::new();
        args.insert("freq".to_string(), Value::Int(11));

        let result = cmd.build_command(&args).unwrap();
        assert_eq!(result, vec![0x11, 0x22, 0x42, 0x44]);
    }

    #[test]
    fn test_build_command_invalid_bcd() {
        let mask = BinMask::try_from("1122??44").unwrap();
        let mut params = BTreeMap::new();
        params.insert(
            "freq".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 0.0,
                multiply: 1.0,
            },
        );

        let cmd = Command {
            command: mask,
            response: None,
            validator: None,
            params,
            returns: BTreeMap::new(),
        };

        let mut args = BTreeMap::new();
        args.insert("freq".to_string(), Value::Int(-1));

        assert!(matches!(
            cmd.build_command(&args),
            Err(CommandError::DataFormat(
                DataFormatError::NegativeNotSupported { .. }
            ))
        ));

        args.insert("freq".to_string(), Value::Int(100));
        assert!(matches!(
            cmd.build_command(&args),
            Err(CommandError::DataFormat(
                DataFormatError::NumberTooLong { .. }
            ))
        ));
    }

    #[test]
    fn test_parse_response_without_response_mask() {
        let command = Command {
            command: BinMask {
                data: vec![0x00],
                masks: vec![],
            },
            response: None,
            validator: None,
            params: BTreeMap::new(),
            returns: {
                let mut returns = BTreeMap::new();
                returns.insert(
                    "value".to_string(),
                    BinaryParam {
                        index: 0,
                        length: 1,
                        format: DataFormat::IntBu,
                        add: 0.0,
                        multiply: 1.0,
                    },
                );
                returns
            },
        };

        assert!(matches!(
            command.validate(),
            Err(CommandError::ReturnValuesWithoutResponse)
        ));
    }

    #[test]
    fn test_parse_response_with_transforms() -> Result<(), CommandError> {
        let command = Command {
            command: BinMask {
                data: vec![0x00],
                masks: vec![],
            },
            response: Some(BinMask {
                data: vec![50],
                masks: vec![(0, 1)],
            }),
            validator: None,
            params: BTreeMap::new(),
            returns: {
                let mut returns = BTreeMap::new();
                returns.insert(
                    "value".to_string(),
                    BinaryParam {
                        index: 0,
                        length: 1,
                        format: DataFormat::IntBu,
                        add: 10.0,
                        multiply: 2.0,
                    },
                );
                returns
            },
        };
        command.validate()?;

        let response = vec![50];
        let result = command.parse_response(&response)?;

        assert_eq!(result.len(), 1);
        assert!(matches!(result.get("value"), Some(20)));
        Ok(())
    }

    #[test]
    fn test_parse_response_multiple_values() -> Result<(), CommandError> {
        let command = Command {
            command: BinMask {
                data: vec![0x00],
                masks: vec![],
            },
            response: Some(BinMask {
                data: vec![0, 0, 0, 0],
                masks: vec![(0, 2), (2, 2)],
            }),
            validator: None,
            params: BTreeMap::new(),
            returns: {
                let mut returns = BTreeMap::new();
                returns.insert(
                    "first".to_string(),
                    BinaryParam {
                        index: 0,
                        length: 2,
                        format: DataFormat::IntBu,
                        add: 0.0,
                        multiply: 1.0,
                    },
                );
                returns.insert(
                    "second".to_string(),
                    BinaryParam {
                        index: 2,
                        length: 2,
                        format: DataFormat::IntBu,
                        add: 0.0,
                        multiply: 1.0,
                    },
                );
                returns
            },
        };

        let response = vec![0x01, 0x02, 0x03, 0x04];
        let result = command.parse_response(&response)?;

        assert_eq!(result.len(), 2);
        assert!(matches!(result.get("first"), Some(258)));
        assert!(matches!(result.get("second"), Some(772)));
        Ok(())
    }

    #[test]
    fn test_parse_response_uncovered_return() {
        let command = Command {
            command: BinMask {
                data: vec![0x00],
                masks: vec![],
            },
            response: Some(BinMask {
                data: vec![0x01, 0x02],
                masks: vec![(0, 1)],
            }),
            validator: None,
            params: BTreeMap::new(),
            returns: {
                let mut returns = BTreeMap::new();
                returns.insert(
                    "value".to_string(),
                    BinaryParam {
                        index: 0,
                        length: 2,
                        format: DataFormat::IntBu,
                        add: 0.0,
                        multiply: 1.0,
                    },
                );
                returns
            },
        };

        assert!(matches!(
            command.validate(),
            Err(CommandError::UncoveredParam)
        ));
    }

    #[test]
    fn test_parse_response_invalid_length() {
        let command = Command {
            command: BinMask {
                data: vec![0x00],
                masks: vec![],
            },
            response: Some(BinMask {
                data: vec![0x01, 0x02],
                masks: vec![],
            }),
            validator: None,
            params: BTreeMap::new(),
            returns: {
                let mut returns = BTreeMap::new();
                returns.insert(
                    "value".to_string(),
                    BinaryParam {
                        index: 0,
                        length: 3,
                        format: DataFormat::IntBu,
                        add: 0.0,
                        multiply: 1.0,
                    },
                );
                returns
            },
        };

        let response = vec![0x01, 0x02];
        assert!(matches!(
            command.parse_response(&response),
            Err(CommandError::InvalidMask)
        ));
    }

    #[test]
    fn test_parse_response_invalid_data() {
        let command = Command {
            command: BinMask {
                data: vec![0x00],
                masks: vec![],
            },
            response: Some(BinMask {
                data: vec![0xFF],
                masks: vec![],
            }),
            validator: None,
            params: BTreeMap::new(),
            returns: {
                let mut returns = BTreeMap::new();
                returns.insert(
                    "value".to_string(),
                    BinaryParam {
                        index: 0,
                        length: 1,
                        format: DataFormat::BcdBu,
                        add: 0.0,
                        multiply: 1.0,
                    },
                );
                returns
            },
        };

        let response = vec![0xFF];
        assert!(matches!(
            command.parse_response(&response),
            Err(CommandError::InvalidArgumentValue(_))
        ));
    }

    #[test]
    fn test_parse_response_wrong_length() {
        let command = Command {
            command: BinMask {
                data: vec![0x00],
                masks: vec![],
            },
            response: Some(BinMask {
                data: vec![0x01],
                masks: vec![],
            }),
            validator: None,
            params: BTreeMap::new(),
            returns: {
                let mut returns = BTreeMap::new();
                returns.insert(
                    "value".to_string(),
                    BinaryParam {
                        index: 0,
                        length: 1,
                        format: DataFormat::IntBu,
                        add: 0.0,
                        multiply: 1.0,
                    },
                );
                returns
            },
        };

        let response = vec![0x01, 0x02];
        assert!(matches!(
            command.parse_response(&response),
            Err(CommandError::InvalidMask)
        ));
    }

    #[test]
    fn test_build_command_with_float_transforms() {
        let mask = BinMask::try_from("1122??44").unwrap();
        let mut params = BTreeMap::new();
        params.insert(
            "freq".to_string(),
            BinaryParam {
                index: 2,
                length: 1,
                format: DataFormat::BcdBu,
                add: 10.5,
                multiply: 2.5,
            },
        );

        let cmd = Command {
            command: mask,
            response: None,
            validator: None,
            params,
            returns: BTreeMap::new(),
        };

        let mut args = BTreeMap::new();
        args.insert("freq".to_string(), Value::Int(11));

        let result = cmd.build_command(&args).unwrap();
        assert_eq!(result, vec![0x11, 0x22, 0x54, 0x44]);
    }

    #[test]
    fn test_parse_response_with_float_transforms() -> Result<(), CommandError> {
        let command = Command {
            command: BinMask {
                data: vec![0x00],
                masks: vec![],
            },
            response: Some(BinMask {
                data: vec![50],
                masks: vec![(0, 1)],
            }),
            validator: None,
            params: BTreeMap::new(),
            returns: {
                let mut returns = BTreeMap::new();
                returns.insert(
                    "value".to_string(),
                    BinaryParam {
                        index: 0,
                        length: 1,
                        format: DataFormat::IntBu,
                        add: 10.5,
                        multiply: 2.5,
                    },
                );
                returns
            },
        };
        command.validate()?;

        let response = vec![50];
        let result = command.parse_response(&response)?;

        assert_eq!(result.len(), 1);
        assert!(matches!(result.get("value"), Some(16)));
        Ok(())
    }
}
