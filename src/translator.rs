use crate::commands::{BinaryParam, Command as RigCommand, CommandValidator};
use crate::{
    data_format::DataFormat,
    omnirig_parser::{Command, EndOfData, RigDescription},
    rig_file::RigFile,
};
use std::collections::HashMap;

pub fn translate_omnirig_to_rig(omnirig: RigDescription) -> RigFile {
    let mut rig_file = RigFile::new();

    // Convert init commands
    for (idx, cmd) in omnirig.init_commands.iter().enumerate() {
        let command_format = convert_command(cmd);
        rig_file
            .init
            .insert(format!("init{}", idx + 1), command_format);
    }

    // Convert parameter commands
    for cmd in omnirig.param_commands.iter() {
        // Try to determine a meaningful name from the command content
        let name = determine_command_name(cmd);
        let command_format = convert_command(cmd);
        rig_file.commands.insert(name, command_format);
    }

    // Convert status commands
    for (idx, cmd) in omnirig.status_commands.iter().enumerate() {
        let command_format = convert_command(cmd);
        rig_file
            .status
            .insert(format!("status{}", idx + 1), command_format);
    }

    rig_file
}

fn convert_command(cmd: &Command) -> RigCommand {
    let validator = match &cmd.end_of_data {
        EndOfData::Length(length) => CommandValidator::ReplyLength(*length),
        EndOfData::String(delimiter) => CommandValidator::ReplyEnd(delimiter.clone()),
    };

    let mut params = HashMap::new();
    if let Some(value) = &cmd.value {
        // Parse value field in format: <start_pos>|<length>|<format>|<multiply>|<add>
        let parts: Vec<&str> = value.split('|').collect();
        if parts.len() >= 3 {
            let index = parts[0].parse().unwrap();
            let length = parts[1].parse().unwrap();
            let format = match parts[2] {
                "vfBcdBU" => DataFormat::BcdBu,
                "vfBcdLU" => DataFormat::BcdLu,
                "vfText" => DataFormat::Text,
                // Add more format mappings as needed
                _ => DataFormat::Text, // Default to text for now
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
                BinaryParam {
                    index,
                    length,
                    format,
                    multiply,
                    add,
                },
            );
        }
    }

    RigCommand {
        command: cmd.command.as_str().try_into().unwrap(),
        validator: Some(validator),
        params,
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
    use crate::{commands::HexMask, omnirig_parser::parse_ini_file};
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
}
