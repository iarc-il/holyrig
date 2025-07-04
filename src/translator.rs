use crate::{
    omnirig_parser::{Command, EndOfData, RigDescription},
    rig_file::{CommandFormat, CommandValidator, RigFile},
};

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

fn convert_command(cmd: &Command) -> CommandFormat {
    let validator = match &cmd.end_of_data {
        EndOfData::Length(length) => {
            CommandValidator::ReplyLength(*length)
        },
        EndOfData::String(delimiter) => {
            CommandValidator::ReplyEnd(delimiter.clone())
        },
    };
    let command_format = CommandFormat {
        command: cmd.command.clone(),
        validator: Some(validator),
        // TODO: Missing validate field
        // validate: cmd.validate.clone(),
    };

    command_format
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
    use crate::omnirig_parser::parse_ini_file;
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
