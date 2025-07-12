use crate::{
    data_format::DataFormat,
    omnirig_parser::{Command, EndOfData, RigDescription},
    rig_file::{RigBinaryParam, RigCommand, RigFile},
};
use std::collections::HashMap;

pub fn translate_omnirig_to_rig(omnirig: RigDescription) -> RigFile {
    let mut rig_file = RigFile::new();

    // Convert init commands
    for cmd in omnirig.init_commands.iter() {
        let command_format = convert_command(cmd);
        rig_file.init.push(command_format);
    }

    // Convert parameter commands
    for cmd in omnirig.param_commands.iter() {
        // Try to determine a meaningful name from the command content
        let name = determine_command_name(cmd);
        let command_format = convert_command(cmd);
        rig_file.commands.insert(name, command_format);
    }

    // Convert status commands
    for cmd in omnirig.status_commands.iter() {
        let command_format = convert_command(cmd);
        rig_file.status.push(command_format);
    }

    rig_file
}

fn convert_command(cmd: &Command) -> RigCommand {
    let mut params = HashMap::new();
    if let Some(value) = &cmd.value {
        // Parse value field in format: <start_pos>|<length>|<format>|<multiply>|<add>
        let parts: Vec<&str> = value.split('|').collect();
        if parts.len() >= 3 {
            let index = parts[0].parse().unwrap();
            let length = parts[1].parse().unwrap();
            let format = match parts[2] {
                "vfBcdBU" => "bcd_bu",
                "vfBcdLU" => "bcd_lu",
                "vfText" => "text",
                // Add more format mappings as needed
                _ => "text", // Default to text for now
            };
            let multiply = if parts.len() > 3 {
                parts[3].parse().unwrap_or(1)
            } else {
                1
            };
            let add = if parts.len() > 4 {
                parts[4].parse().unwrap_or(0)
            } else {
                0
            };

            params.insert(
                "value".to_string(),
                RigBinaryParam {
                    index,
                    length,
                    format: DataFormat::try_from(format).unwrap(),
                    multiply,
                    add,
                },
            );
        }
    }

    let (reply_length, reply_end) = match &cmd.end_of_data {
        EndOfData::Length(length) => (Some(*length), None),
        EndOfData::String(delimiter) => (None, Some(delimiter.clone())),
    };

    RigCommand {
        command: cmd.command.clone(),
        reply_length,
        reply_end,
        response: cmd.validate.clone(),
        params,
        returns: HashMap::new(),
    }
}

fn determine_command_name(cmd: &Command) -> String {
    // Try to extract meaningful name from command content
    // This is a heuristic approach - could be improved based on actual command patterns
    if cmd.command.contains("FREQ") || cmd.command.contains("freq") {
        "set_freq".to_string()
    } else if cmd.command.contains("MODE") || cmd.command.contains("mode") {
        "set_mode".to_string()
    } else if cmd.command.contains("PTT") || cmd.command.contains("ptt") {
        "set_ptt".to_string()
    } else {
        // Fallback to a generic name if we can't determine the purpose
        format!("cmd_{}", cmd.command.chars().take(8).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use crate::{data_format::DataFormat, omnirig_parser::parse_ini_file};
    use anyhow::Result;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_convert() -> Result<()> {
        let test_file = PathBuf::from("rig_files/IC-705.ini");
        let rig_desc = parse_ini_file(test_file)?;
        let rig_data = translate_omnirig_to_rig(rig_desc);
        println!("Rig data: {rig_data:?}");
        Ok(())
    }

    #[test]
    fn test_convert_command_with_value() {
        let cmd = Command {
            command: "FEFE94E0140900FD".to_string(),
            end_of_data: EndOfData::Length(15),
            validate: None,
            value: Some("6|2|vfBcdBU|4|-127".to_string()),
            values: vec![],
            flags: vec![],
        };

        let cmd_format = convert_command(&cmd);
        assert_eq!(cmd_format.command.as_str(), "FEFE94E0140900FD");
        assert!(matches!(cmd_format.reply_length, Some(15)));

        let param = cmd_format.params.get("value").unwrap();
        assert_eq!(param.index, 6);
        assert_eq!(param.length, 2);
        assert!(matches!(param.format, DataFormat::BcdBu));
        assert_eq!(param.multiply, 4);
        assert_eq!(param.add, -127);
    }

    #[test]
    fn test_convert_command_without_value() {
        let cmd = Command {
            command: "FEFE94E02100000000FD".to_string(),
            end_of_data: EndOfData::String("FEFEE094FBFD".to_string()),
            validate: None,
            value: None,
            values: vec![],
            flags: vec![],
        };

        let cmd_format = convert_command(&cmd);
        assert_eq!(cmd_format.command.as_str(), "FEFE94E02100000000FD");
        assert!(cmd_format.reply_end == Some("FEFEE094FBFD".to_string()));
        assert!(cmd_format.params.is_empty());
    }
}
