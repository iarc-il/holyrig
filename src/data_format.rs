use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum DataFormatError {
    InvalidName(String),
    NumberTooLong { value: i32, length: usize },
    NegativeNotSupported { value: i32, format: DataFormat },
}

impl Display for DataFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataFormatError::InvalidName(name) => write!(f, "Invalid format name: {name}"),
            DataFormatError::NumberTooLong { value, length } => {
                write!(f, "Number {value} is too long to fit in {length} bytes")
            }
            DataFormatError::NegativeNotSupported { value, format } => {
                write!(
                    f,
                    "Negative number {value} is not supported by format {format}"
                )
            }
        }
    }
}

impl std::error::Error for DataFormatError {}

#[derive(Debug, Serialize, Deserialize)]
pub enum DataFormat {
    BcdBs,
    BcdBu,
    BcdLs,
    BcdLu,
    IntBs,
    IntBu,
    IntLs,
    IntLu,
    Text,
}

impl Display for DataFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = match self {
            DataFormat::BcdBs => "bcd_bs",
            DataFormat::BcdBu => "bcd_bu",
            DataFormat::BcdLs => "bcd_ls",
            DataFormat::BcdLu => "bcd_lu",
            DataFormat::IntBs => "int_bs",
            DataFormat::IntBu => "int_bu",
            DataFormat::IntLs => "int_ls",
            DataFormat::IntLu => "int_lu",
            DataFormat::Text => "text",
        };
        write!(f, "{result}")
    }
}

impl TryFrom<&str> for DataFormat {
    type Error = DataFormatError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let result = match value {
            "bcd_bs" => DataFormat::BcdBs,
            "bcd_bu" => DataFormat::BcdBu,
            "bcd_ls" => DataFormat::BcdLs,
            "bcd_lu" => DataFormat::BcdLu,
            "int_bs" => DataFormat::IntBs,
            "int_bu" => DataFormat::IntBu,
            "int_ls" => DataFormat::IntLs,
            "int_lu" => DataFormat::IntLu,
            "text" => DataFormat::Text,
            _ => {
                return Err(DataFormatError::InvalidName(value.to_string()));
            }
        };
        Ok(result)
    }
}

impl DataFormat {
    pub fn encode(&self, value: i32, length: usize) -> Result<Vec<u8>, DataFormatError> {
        match self {
            DataFormat::BcdBs => Self::encode_bcd_bs(value, length),
            DataFormat::BcdBu => Self::encode_bcd_bu(value, length),
            DataFormat::BcdLs => Self::encode_bcd_ls(value, length),
            DataFormat::BcdLu => Self::encode_bcd_lu(value, length),
            DataFormat::IntBs => Self::encode_int_bs(value, length),
            DataFormat::IntBu => Self::encode_int_bu(value as u32, length),
            DataFormat::IntLs => Self::encode_int_ls(value, length),
            DataFormat::IntLu => Self::encode_int_lu(value as u32, length),
            DataFormat::Text => Self::encode_text(value, length),
        }
    }

    fn encode_bcd_bs(value: i32, length: usize) -> Result<Vec<u8>, DataFormatError> {
        let mut result = vec![0; length];
        let abs_value = value.abs();
        let mut digits = abs_value.to_string();

        // TODO: convert to digits without formatting strings
        if digits.len() % 2 != 0 {
            digits = format!("0{digits}");
        }

        let mut bcd_bytes = Vec::new();
        for chunk in digits.as_bytes().chunks(2) {
            let high = (chunk[0] - b'0') << 4;
            let low = chunk[1] - b'0';
            bcd_bytes.push(high | low);
        }

        if bcd_bytes.len() > length {
            return Err(DataFormatError::NumberTooLong { value, length });
        }

        let start = result.len() - bcd_bytes.len();
        result[start..].copy_from_slice(&bcd_bytes);

        if value < 0 {
            result[0] = 0xFF;
        }

        Ok(result)
    }

    fn encode_bcd_bu(value: i32, length: usize) -> Result<Vec<u8>, DataFormatError> {
        if value < 0 {
            return Err(DataFormatError::NegativeNotSupported {
                value,
                format: DataFormat::BcdBu,
            });
        }
        Self::encode_bcd_bs(value, length)
    }

    fn encode_bcd_ls(value: i32, length: usize) -> Result<Vec<u8>, DataFormatError> {
        let mut result = Self::encode_bcd_bs(value, length)?;
        result.reverse();
        Ok(result)
    }

    fn encode_bcd_lu(value: i32, length: usize) -> Result<Vec<u8>, DataFormatError> {
        if value < 0 {
            return Err(DataFormatError::NegativeNotSupported {
                value,
                format: DataFormat::BcdLu,
            });
        }
        Self::encode_bcd_ls(value, length)
    }

    fn encode_int_bs(value: i32, length: usize) -> Result<Vec<u8>, DataFormatError> {
        let mut result = vec![0; length];
        let bytes = value.to_be_bytes();
        if bytes.len() > length {
            return Err(DataFormatError::NumberTooLong { value, length });
        }
        let start = result.len() - bytes.len();
        result[start..].copy_from_slice(&bytes);
        if value < 0 {
            for byte in result.iter_mut().take(start) {
                *byte = 0xFF;
            }
        }
        Ok(result)
    }

    fn encode_int_bu(value: u32, length: usize) -> Result<Vec<u8>, DataFormatError> {
        let mut result = vec![0; length];
        let bytes = value.to_be_bytes();
        if bytes.len() > length {
            return Err(DataFormatError::NumberTooLong {
                value: value as i32,
                length,
            });
        }
        let start = result.len() - bytes.len();
        result[start..].copy_from_slice(&bytes);
        Ok(result)
    }

    fn encode_int_ls(value: i32, length: usize) -> Result<Vec<u8>, DataFormatError> {
        let mut result = vec![0; length];
        let bytes = value.to_le_bytes();
        if bytes.len() > length {
            return Err(DataFormatError::NumberTooLong { value, length });
        }
        result[..bytes.len()].copy_from_slice(&bytes);
        if value < 0 {
            for byte in result.iter_mut().take(length).skip(bytes.len()) {
                *byte = 0xFF;
            }
        }
        Ok(result)
    }

    fn encode_int_lu(value: u32, length: usize) -> Result<Vec<u8>, DataFormatError> {
        let mut result = vec![0; length];
        let bytes = value.to_le_bytes();
        if bytes.len() > length {
            return Err(DataFormatError::NumberTooLong {
                value: value as i32,
                length,
            });
        }
        result[..bytes.len()].copy_from_slice(&bytes);
        Ok(result)
    }

    fn encode_text(value: i32, length: usize) -> Result<Vec<u8>, DataFormatError> {
        let text = value.to_string();
        if text.len() > length {
            return Err(DataFormatError::NumberTooLong { value, length });
        }
        let mut result = vec![b'0'; length];
        let start = length.saturating_sub(text.len());
        result[start..].copy_from_slice(text.as_bytes());
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bcd_formats() -> Result<(), DataFormatError> {
        let expected_bcd_b = vec![0x00, 0x00, 0x04, 0x18];
        assert_eq!(DataFormat::BcdBs.encode(418, 4)?, expected_bcd_b);
        assert_eq!(DataFormat::BcdBu.encode(418, 4)?, expected_bcd_b);

        let expected_bcd_l = vec![0x18, 0x04, 0x00, 0x00];
        assert_eq!(DataFormat::BcdLs.encode(418, 4)?, expected_bcd_l);
        assert_eq!(DataFormat::BcdLu.encode(418, 4)?, expected_bcd_l);

        let expected_bcd_bs_neg = vec![0xFF, 0x00, 0x04, 0x18];
        assert_eq!(DataFormat::BcdBs.encode(-418, 4)?, expected_bcd_bs_neg);

        let expected_bcd_ls_neg = vec![0x18, 0x04, 0x00, 0xFF];
        assert_eq!(DataFormat::BcdLs.encode(-418, 4)?, expected_bcd_ls_neg);
        Ok(())
    }

    #[test]
    fn test_int_formats() -> Result<(), DataFormatError> {
        let expected_int_b = vec![0x00, 0x00, 0x01, 0xA2];
        assert_eq!(DataFormat::IntBs.encode(418, 4)?, expected_int_b);
        assert_eq!(DataFormat::IntBu.encode(418, 4)?, expected_int_b);

        let expected_int_l = vec![0xA2, 0x01, 0x00, 0x00];
        assert_eq!(DataFormat::IntLs.encode(418, 4)?, expected_int_l);
        assert_eq!(DataFormat::IntLu.encode(418, 4)?, expected_int_l);

        let expected_int_bs_neg = vec![0xFF, 0xFF, 0xFE, 0x5E];
        assert_eq!(DataFormat::IntBs.encode(-418, 4)?, expected_int_bs_neg);

        let expected_int_ls_neg = vec![0x5E, 0xFE, 0xFF, 0xFF];
        assert_eq!(DataFormat::IntLs.encode(-418, 4)?, expected_int_ls_neg);
        Ok(())
    }

    #[test]
    fn test_text_format() -> Result<(), DataFormatError> {
        assert_eq!(DataFormat::Text.encode(5, 4)?, vec![0x30, 0x30, 0x30, 0x35]);
        assert_eq!(DataFormat::Text.encode(0, 4)?, vec![0x30, 0x30, 0x30, 0x30]);
        assert_eq!(DataFormat::Text.encode(7, 1)?, vec![0x37]);
        assert_eq!(
            DataFormat::Text.encode(42, 8)?,
            vec![0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x34, 0x32]
        );
        Ok(())
    }

    #[test]
    fn test_text_format_overflow() {
       tassert!(matches!(
            DataFormat::Text.encode(12345, 4),
            Err(DataFormatError::NumberTooLong {
                value: 12345,
                length: 4
            })
        ));
    }

    #[test]
    fn test_text_format_negative_overflow() {
        assert!(matches!(
            DataFormat::Text.encode(-12345, 4),
            Err(DataFormatError::NumberTooLong {
                value: -12345,
                length: 4
            })
        ));
    }

    #[test]
    fn test_unsigned_negative() {
        assert!(matches!(
            DataFormat::BcdBu.encode(-418, 4),
            Err(DataFormatError::NegativeNotSupported {
                value: -418,
                format: DataFormat::BcdBu
            })
        ));
        assert!(matches!(
            DataFormat::BcdLu.encode(-418, 4),
            Err(DataFormatError::NegativeNotSupported {
                value: -418,
                format: DataFormat::BcdLu
            })
        ));
    }
}
