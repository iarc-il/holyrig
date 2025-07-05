use serde::{Deserialize, Serialize};

// Value |     418     |    -418
// ------|-------------|------------
// BcdBs | 00.00.04.18 | FF.00.04.18
// BcdBu | 00.00.04.18 | -
// BcdLs | 18.04.00.00 | 18.04.00.FF
// BcdLu | 18.04.00.00 | -
// IntBs | 00.00.01.A2 | FF.FF.FE.5E
// IntBu | 00.00.01.A2 | -
// IntLs | A2.01.00.00 | 5E.FE.FF.FF
// IntLu | A2.01.00.00 | -
// Text  | 30.34.31.38 | 2D.34.31.38

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

impl DataFormat {
    pub fn encode(&self, value: i32, length: usize) -> Vec<u8> {
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

    fn encode_bcd_bs(value: i32, length: usize) -> Vec<u8> {
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

        let start = result.len() - bcd_bytes.len();
        result[start..].copy_from_slice(&bcd_bytes);

        if value < 0 {
            result[0] = 0xFF;
        }

        result
    }

    fn encode_bcd_bu(value: i32, length: usize) -> Vec<u8> {
        if value < 0 {
            panic!("BcdBu format does not support negative numbers");
        }
        Self::encode_bcd_bs(value, length)
    }

    fn encode_bcd_ls(value: i32, length: usize) -> Vec<u8> {
        let mut result = Self::encode_bcd_bs(value, length);
        result.reverse();
        result
    }

    fn encode_bcd_lu(value: i32, length: usize) -> Vec<u8> {
        if value < 0 {
            panic!("BcdLu format does not support negative numbers");
        }
        Self::encode_bcd_ls(value, length)
    }

    fn encode_int_bs(value: i32, length: usize) -> Vec<u8> {
        let mut result = vec![0; length];
        let bytes = value.to_be_bytes();
        let start = result.len() - bytes.len();
        result[start..].copy_from_slice(&bytes);
        if value < 0 {
            for byte in result.iter_mut().take(start) {
                *byte = 0xFF;
            }
        }
        result
    }

    fn encode_int_bu(value: u32, length: usize) -> Vec<u8> {
        let mut result = vec![0; length];
        let bytes = value.to_be_bytes();
        let start = result.len() - bytes.len();
        result[start..].copy_from_slice(&bytes);
        result
    }

    fn encode_int_ls(value: i32, length: usize) -> Vec<u8> {
        let mut result = vec![0; length];
        let bytes = value.to_le_bytes();
        result[..bytes.len()].copy_from_slice(&bytes);
        if value < 0 {
            for byte in result.iter_mut().take(length).skip(bytes.len()) {
                *byte = 0xFF;
            }
        }
        result
    }

    fn encode_int_lu(value: u32, length: usize) -> Vec<u8> {
        let mut result = vec![0; length];
        let bytes = value.to_le_bytes();
        result[..bytes.len()].copy_from_slice(&bytes);
        result
    }

    fn encode_text(value: i32, length: usize) -> Vec<u8> {
        let text = value.to_string();
        if text.len() > length {
            panic!("Number {value} is too long to fit in {length} bytes");
        }
        let mut result = vec![b'0'; length];
        let start = length.saturating_sub(text.len());
        result[start..].copy_from_slice(text.as_bytes());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bcd_formats() {
        let expected_bcd_b = vec![0x00, 0x00, 0x04, 0x18];
        assert_eq!(DataFormat::BcdBs.encode(418, 4), expected_bcd_b);
        assert_eq!(DataFormat::BcdBu.encode(418, 4), expected_bcd_b);

        let expected_bcd_l = vec![0x18, 0x04, 0x00, 0x00];
        assert_eq!(DataFormat::BcdLs.encode(418, 4), expected_bcd_l);
        assert_eq!(DataFormat::BcdLu.encode(418, 4), expected_bcd_l);

        let expected_bcd_bs_neg = vec![0xFF, 0x00, 0x04, 0x18];
        assert_eq!(DataFormat::BcdBs.encode(-418, 4), expected_bcd_bs_neg);

        let expected_bcd_ls_neg = vec![0x18, 0x04, 0x00, 0xFF];
        assert_eq!(DataFormat::BcdLs.encode(-418, 4), expected_bcd_ls_neg);
    }

    #[test]
    fn test_int_formats() {
        let expected_int_b = vec![0x00, 0x00, 0x01, 0xA2];
        assert_eq!(DataFormat::IntBs.encode(418, 4), expected_int_b);
        assert_eq!(DataFormat::IntBu.encode(418, 4), expected_int_b);

        let expected_int_l = vec![0xA2, 0x01, 0x00, 0x00];
        assert_eq!(DataFormat::IntLs.encode(418, 4), expected_int_l);
        assert_eq!(DataFormat::IntLu.encode(418, 4), expected_int_l);

        // Test negative number -418
        let expected_int_bs_neg = vec![0xFF, 0xFF, 0xFE, 0x5E];
        assert_eq!(DataFormat::IntBs.encode(-418, 4), expected_int_bs_neg);

        let expected_int_ls_neg = vec![0x5E, 0xFE, 0xFF, 0xFF];
        assert_eq!(DataFormat::IntLs.encode(-418, 4), expected_int_ls_neg);
    }

    #[test]
    fn test_text_format() {
        assert_eq!(
            DataFormat::Text.encode(418, 4),
            vec![0x30, 0x34, 0x31, 0x38]
        );
        assert_eq!(
            DataFormat::Text.encode(-418, 4),
            vec![0x2D, 0x34, 0x31, 0x38]
        );
    }

    #[test]
    fn test_text_format_edge_cases() {
        assert_eq!(DataFormat::Text.encode(5, 4), vec![0x30, 0x30, 0x30, 0x35]);

        assert_eq!(DataFormat::Text.encode(0, 4), vec![0x30, 0x30, 0x30, 0x30]);

        assert_eq!(DataFormat::Text.encode(7, 1), vec![0x37]);

        assert_eq!(
            DataFormat::Text.encode(42, 8),
            vec![0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x34, 0x32]
        );
    }

    #[test]
    #[should_panic(expected = "Number 12345 is too long to fit in 4 bytes")]
    fn test_text_format_overflow() {
        // Number longer than buffer should panic
        DataFormat::Text.encode(12345, 4);
    }

    #[test]
    #[should_panic(expected = "Number -12345 is too long to fit in 4 bytes")]
    fn test_text_format_negative_overflow() {
        // Negative number longer than buffer should panic
        DataFormat::Text.encode(-12345, 4);
    }

    #[test]
    #[should_panic(expected = "BcdBu format does not support negative numbers")]
    fn test_bcd_bu_negative() {
        DataFormat::BcdBu.encode(-418, 4);
    }

    #[test]
    #[should_panic(expected = "BcdLu format does not support negative numbers")]
    fn test_bcd_lu_negative() {
        DataFormat::BcdLu.encode(-418, 4);
    }
}
