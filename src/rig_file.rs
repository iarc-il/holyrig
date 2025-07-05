use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::commands::CommandFormat;

#[derive(Debug, Serialize, Deserialize)]
pub struct General {
    pub r#type: String,
    pub version: u8,
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
    use crate::commands::{HexMask, ParseError};
    use crate::data_format::DataFormat;
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
