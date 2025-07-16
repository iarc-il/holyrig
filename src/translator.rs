use anyhow::{Result, bail};

use crate::{
    data_format::DataFormat,
    omnirig_parser::{Command, EndOfData, RigDescription},
    rig_file::{RigBinaryParam, RigCommand, RigFile},
};
use std::collections::HashMap;

#[derive(Debug)]
struct BinaryParamLocation {
    offset: usize,
    length: usize,
}

fn tokenize_command(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();

    for c in command.chars() {
        match c {
            c if c.is_ascii_hexdigit() => {
                current_token.push(c);
                if current_token.len() == 2 {
                    tokens.push(current_token);
                    current_token = String::new();
                }
            }
            '.' => {
                if !current_token.is_empty() {
                    tokens.push(current_token);
                    current_token = String::new();
                }
                tokens.push(".".to_string());
            }
            _ => {
                panic!("Unknown char {c}");
            }
        }
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    tokens
}

fn find_param_location(commands: &[Command]) -> Option<BinaryParamLocation> {
    if commands.len() < 2 {
        return None;
    }

    let mut first_tokens = tokenize_command(&commands[0].command);
    first_tokens.retain(|token| token != ".");
    let mut start_diff = first_tokens.len();
    let mut end_diff = 0;

    for cmd in &commands[1..] {
        let mut tokens = tokenize_command(&cmd.command);
        tokens.retain(|token| token != ".");
        let mut common_prefix = 0;
        let mut common_suffix = 0;

        for (i, (a, b)) in first_tokens.iter().zip(tokens.iter()).enumerate() {
            if a != b {
                break;
            }
            common_prefix = i + 1;
        }

        let first_rev: Vec<_> = first_tokens.iter().rev().collect();
        let tokens_rev: Vec<_> = tokens.iter().rev().collect();
        for (i, (a, b)) in first_rev.iter().zip(tokens_rev.iter()).enumerate() {
            if a != b {
                break;
            }
            common_suffix = i + 1;
        }

        start_diff = start_diff.min(common_prefix);
        end_diff = end_diff.max(common_suffix);
    }

    let offset = start_diff;
    let length = first_tokens.len() - start_diff - end_diff;

    Some(BinaryParamLocation { offset, length })
}

fn find_mode_param_location(commands: &[Command]) -> Option<BinaryParamLocation> {
    let mode_commands: Vec<_> = commands
        .iter()
        .filter(|cmd| extract_mode_params(&cmd.name).is_some())
        .cloned()
        .collect();

    find_param_location(&mode_commands)
}

fn find_toggle_param_location(
    commands: &[Command],
    command_type: &str,
) -> Option<BinaryParamLocation> {
    let toggle_commands: Vec<_> = commands
        .iter()
        .filter(|cmd| extract_toggle_params(&cmd.name, command_type).is_some())
        .cloned()
        .collect();

    find_param_location(&toggle_commands)
}

fn extract_mode_params(cmd_name: &str) -> Option<(String, Option<String>)> {
    let name = cmd_name.strip_prefix("pm")?;
    match name.to_uppercase().as_str() {
        "CW_U" => Some(("CW".to_string(), Some("Upper".to_string()))),
        "CW_L" => Some(("CW".to_string(), Some("Lower".to_string()))),
        "SSB_U" => Some(("SSB".to_string(), Some("Upper".to_string()))),
        "SSB_L" => Some(("SSB".to_string(), Some("Lower".to_string()))),
        "DIG_U" => Some(("DIG".to_string(), Some("Upper".to_string()))),
        "DIG_L" => Some(("DIG".to_string(), Some("Lower".to_string()))),
        "AM" => Some(("AM".to_string(), None)),
        "FM" => Some(("FM".to_string(), None)),
        _ => None,
    }
}

fn extract_toggle_params(name: &str, command_type: &str) -> Option<bool> {
    let prefix = format!("pm{}", command_type.to_lowercase());
    let name = name.to_lowercase();
    if name.to_lowercase() == format!("{prefix}on") {
        Some(true)
    } else if name.to_lowercase() == format!("{prefix}off") {
        Some(false)
    } else {
        None
    }
}

fn insert_question_marks(command: &str, offset: usize, length: usize) -> String {
    let mut byte_offset = 0;
    let mut result = vec![];
    for token in tokenize_command(command) {
        if token == "." {
            result.push(token);
            continue;
        }
        if byte_offset >= offset && byte_offset < offset + length {
            result.push("??".to_string());
        } else {
            result.push(token);
        }
        byte_offset += 1;
    }

    result.join("")
}

struct CommandTranslation {
    name: String,
    mode_params: Option<(String, Option<String>)>,
    toggle_param: Option<(String, bool)>,
}

fn determine_command_name(cmd: &Command) -> Result<CommandTranslation> {
    if let Some(mode_params) = extract_mode_params(&cmd.name) {
        return Ok(CommandTranslation {
            name: "set_mode".to_string(),
            mode_params: Some(mode_params),
            toggle_param: None,
        });
    }

    // Check for various toggle commands
    for (command_type, schema_name) in &[
        ("split", "set_split"),
        ("rit", "set_rit"),
        ("xit", "set_xit"),
    ] {
        if let Some(toggle_value) = extract_toggle_params(&cmd.name, command_type) {
            return Ok(CommandTranslation {
                name: schema_name.to_string(),
                mode_params: None,
                toggle_param: Some((command_type.to_lowercase(), toggle_value)),
            });
        }
    }

    let name = match cmd.name.to_lowercase().as_str() {
        "pmfreq" | "pmfreqa" | "pmfreqb" => "set_freq",
        "pmpitch" => "cw_pitch",
        "pmritoffset" => "rit_offset",
        "pmrit0" => "clear_rit",
        "pmvfoaa" | "pmvfoab" | "pmvfoba" | "pmvfobb" | "pmvfoa" | "pmvfob" => "set_vfo",
        "pmvfoequal" => "vfo_equal",
        "pmvfoswap" => "vfo_swap",
        "pmrx" => "set_rx",
        "pmtx" => "set_tx",
        _ => bail!("Unknown command: {}", cmd.name),
    };

    Ok(CommandTranslation {
        name: name.to_string(),
        mode_params: None,
        toggle_param: None,
    })
}

fn convert_command(cmd: &Command) -> RigCommand {
    let mut params = HashMap::new();

    if let Some(value) = &cmd.value {
        let parts: Vec<&str> = value.split('|').collect();
        if parts.len() >= 3 {
            let start: u32 = parts[0].parse().unwrap_or(0);
            let length: u32 = parts[1].parse().unwrap_or(0);
            let format = match parts[2] {
                "vfBcdLU" => DataFormat::BcdLu,
                "vfBcdLS" => DataFormat::BcdLs,
                "vfBinL" => DataFormat::IntLu,
                "vfBinB" => DataFormat::IntBu,
                "vfText" => DataFormat::Text,
                _ => DataFormat::Text,
            };
            let multiply = if parts.len() > 3 {
                parts[3].parse().unwrap_or(1.0)
            } else {
                1.0
            };
            let add = if parts.len() > 4 {
                parts[4].parse().unwrap_or(0.0)
            } else {
                0.0
            };

            let param = RigBinaryParam {
                index: start,
                length,
                format,
                multiply,
                add,
            };
            params.insert("value".to_string(), param);
        }
    }

    // Convert flags to return parameters
    for flag in &cmd.flags {
        // Find the position and length of the first different bit between mask and bits
        let mask_bytes = hex_string_to_bytes(&flag.mask);
        let bits_bytes = hex_string_to_bytes(&flag.bits);

        if let (Some(mask_bytes), Some(bits_bytes)) = (mask_bytes, bits_bytes) {
            if let Some((index, length)) = find_different_bits(&mask_bytes, &bits_bytes) {
                let param = RigBinaryParam {
                    index: index as u32,
                    length: length as u32,
                    format: DataFormat::IntLu,
                    multiply: 1.0,
                    add: 0.0,
                };
                returns.insert(flag.param.clone(), param);
            }
        }
    }

    let (reply_length, reply_end) = match &cmd.end_of_data {
        EndOfData::Length(length) => (Some(*length), None),
        EndOfData::String(reply_end) => (None, Some(reply_end.clone())),
    };

    let mut command = cmd.command.clone();
    for param in params.values() {
        command = insert_question_marks(&command, param.index as usize, param.length as usize);
    }

    RigCommand {
        command,
        response: cmd.validate.clone(),
        reply_length,
        reply_end,
        params,
        returns,
    }
}

fn hex_string_to_bytes(hex: &str) -> Option<Vec<u8>> {
    // Remove dots and whitespace
    let hex = hex.replace(['.', ' '], "");

    if hex.len() % 2 != 0 {
        return None;
    }

    let mut bytes = Vec::new();
    for i in (0..hex.len()).step_by(2) {
        if let Ok(byte) = u8::from_str_radix(&hex[i..i + 2], 16) {
            bytes.push(byte);
        } else {
            return None;
        }
    }
    Some(bytes)
}

fn find_different_bits(mask: &[u8], bits: &[u8]) -> Option<(usize, usize)> {
    if mask.len() != bits.len() {
        return None;
    }

    let mut start_index = None;
    let mut end_index = None;

    for (i, (mask_byte, bits_byte)) in mask.iter().zip(bits.iter()).enumerate() {
        if mask_byte != bits_byte {
            if start_index.is_none() {
                start_index = Some(i);
            }
            end_index = Some(i);
        }
    }

    match (start_index, end_index) {
        (Some(start), Some(end)) => Some((start, end - start + 1)),
        _ => None,
    }
}

pub fn translate_omnirig_to_rig(omnirig: RigDescription) -> Result<RigFile> {
    let mut rig_file = RigFile::new();

    let mode_param_location = find_mode_param_location(&omnirig.param_commands);

    // Find parameter locations for all toggle commands
    let toggle_locations: HashMap<_, _> = ["split", "rit", "xit"]
        .iter()
        .filter_map(|&cmd_type| {
            find_toggle_param_location(&omnirig.param_commands, cmd_type).map(|loc| (cmd_type, loc))
        })
        .collect();

    for cmd in omnirig.init_commands.iter() {
        let command_format = convert_command(cmd);
        rig_file.init.push(command_format);
    }

    for cmd in omnirig.param_commands.iter() {
        let translation = determine_command_name(cmd)?;
        let mut command_format = convert_command(cmd);

        if translation.mode_params.is_some() {
            if let Some(loc) = &mode_param_location {
                let mode_param = RigBinaryParam {
                    index: loc.offset as u32,
                    length: loc.length as u32,
                    format: DataFormat::Text,
                    multiply: 1.0,
                    add: 0.0,
                };
                command_format.params.insert("mode".to_string(), mode_param);
                command_format.command =
                    insert_question_marks(&command_format.command, loc.offset, loc.length);
            }
        }

        if let Some((param_type, _toggle_value)) = &translation.toggle_param {
            if let Some(loc) = toggle_locations.get(param_type.as_str()) {
                let toggle_param = RigBinaryParam {
                    index: loc.offset as u32,
                    length: loc.length as u32,
                    format: DataFormat::IntLu,
                    multiply: 1.0,
                    add: 0.0,
                };
                command_format
                    .params
                    .insert(param_type.clone(), toggle_param);
                command_format.command =
                    insert_question_marks(&command_format.command, loc.offset, loc.length);
            }
        }

        rig_file.commands.insert(translation.name, command_format);
    }

    for cmd in omnirig.status_commands.iter() {
        let command_format = convert_command(cmd);
        rig_file.status.push(command_format);
    }

    Ok(rig_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::omnirig_parser::EndOfData;

    #[test]
    fn test_find_param_location() {
        let commands = vec![
            Command {
                name: "pmCW_U".to_string(),
                command: "FEFE94E01201FD".to_string(),
                end_of_data: EndOfData::Length(0),
                validate: None,
                value: None,
                values: vec![],
                flags: vec![],
            },
            Command {
                name: "pmCW_L".to_string(),
                command: "FEFE94E01202FD".to_string(),
                end_of_data: EndOfData::Length(0),
                validate: None,
                value: None,
                values: vec![],
                flags: vec![],
            },
        ];

        let location = find_param_location(&commands).unwrap();
        assert_eq!(location.offset, 5);
        assert_eq!(location.length, 1);
    }

    #[test]
    fn test_find_toggle_param_location() {
        let test_cases = vec![
            (
                "Split",
                vec![
                    ("pmSplitOn", "FEFE94E01901FD"),
                    ("pmSplitOff", "FEFE94E01900FD"),
                ],
            ),
            (
                "Rit",
                vec![
                    ("pmRitOn", "FEFE94E01A01FD"),
                    ("pmRitOff", "FEFE94E01A00FD"),
                ],
            ),
            (
                "Xit",
                vec![
                    ("pmXitOn", "FEFE94E01B01FD"),
                    ("pmXitOff", "FEFE94E01B00FD"),
                ],
            ),
        ];

        for (cmd_type, cmds) in test_cases {
            let commands: Vec<_> = cmds
                .into_iter()
                .map(|(name, cmd)| Command {
                    name: name.to_string(),
                    command: cmd.to_string(),
                    end_of_data: EndOfData::Length(0),
                    validate: None,
                    value: None,
                    values: vec![],
                    flags: vec![],
                })
                .collect();

            let location = find_toggle_param_location(&commands, cmd_type).unwrap();
            assert_eq!(location.offset, 5);
            assert_eq!(location.length, 1);
        }
    }

    #[test]
    fn test_command_name_mapping() -> Result<()> {
        let test_cases = vec![
            ("pmFreq", "set_freq"),
            ("pmFreqA", "set_freq"),
            ("pmFreqB", "set_freq"),
            ("pmPitch", "cw_pitch"),
            ("pmRitOffset", "rit_offset"),
            ("pmRit0", "clear_rit"),
            ("pmRitOn", "set_rit"),
            ("pmRitOff", "set_rit"),
            ("pmVfoAA", "set_vfo"),
            ("pmVfoAB", "set_vfo"),
            ("pmVfoBA", "set_vfo"),
            ("pmVfoBB", "set_vfo"),
            ("pmVfoA", "set_vfo"),
            ("pmVfoB", "set_vfo"),
            ("pmVfoEqual", "vfo_equal"),
            ("pmVfoSwap", "vfo_swap"),
            ("pmSplitOn", "set_split"),
            ("pmSplitOff", "set_split"),
            ("pmXitOn", "set_xit"),
            ("pmXitOff", "set_xit"),
            ("pmCW_U", "set_mode"),
            ("pmCW_L", "set_mode"),
            ("pmSSB_U", "set_mode"),
            ("pmSSB_L", "set_mode"),
            ("pmDIG_U", "set_mode"),
            ("pmDIG_L", "set_mode"),
            ("pmAM", "set_mode"),
            ("pmFM", "set_mode"),
        ];

        for (input, expected) in test_cases {
            let cmd = Command {
                command: "test".to_string(),
                end_of_data: EndOfData::Length(0),
                validate: None,
                value: None,
                values: vec![],
                flags: vec![],
                name: input.to_string(),
            };
            assert_eq!(determine_command_name(&cmd)?.name, expected);
        }
        Ok(())
    }

    #[test]
    fn test_toggle_params_extraction() -> Result<()> {
        let test_cases = vec![
            (
                "Split",
                vec![
                    ("pmSplitOn", Some(true)),
                    ("pmSplitOff", Some(false)),
                    ("pmFreq", None),
                ],
            ),
            (
                "Rit",
                vec![
                    ("pmRitOn", Some(true)),
                    ("pmRitOff", Some(false)),
                    ("pmFreq", None),
                ],
            ),
            (
                "Xit",
                vec![
                    ("pmXitOn", Some(true)),
                    ("pmXitOff", Some(false)),
                    ("pmFreq", None),
                ],
            ),
        ];

        for (cmd_type, cases) in test_cases {
            for (input, expected) in cases {
                assert_eq!(
                    extract_toggle_params(input, cmd_type),
                    expected,
                    "Failed for {cmd_type} with input {input}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn test_insert_question_marks() {
        let test_cases = vec![
            ("FEFE94E0.25.00.FD", 2, 2, "FEFE????.25.00.FD"),
            ("FEFE94E0.25.00.FD", 6, 1, "FEFE94E0.25.00.??"),
            ("FEFE94E0.25.00.000000.FD", 8, 2, "FEFE94E0.25.00.0000??.??"),
        ];

        for (input, offset, length, expected) in test_cases {
            let command = insert_question_marks(input, offset, length);
            assert_eq!(command, expected, "Failed for input: {input}");
        }
    }
}
