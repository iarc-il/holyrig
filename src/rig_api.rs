use std::collections::HashMap;

use crate::{
    commands::{Command, CommandError, Value},
    rig_file::RigFile,
};

#[derive(Debug)]
pub enum CommandType {
    Named(String),
    Init(usize),
    Status(usize),
}

impl std::fmt::Display for CommandType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandType::Named(name) => write!(f, "command '{name}'"),
            CommandType::Init(index) => write!(f, "init command at index {index}"),
            CommandType::Status(index) => write!(f, "status command at index {index}"),
        }
    }
}

#[derive(Debug)]
pub enum RigApiError {
    Command(CommandError),
    CommandNotFound(CommandType),
    InvalidInit {
        command_index: usize,
        reason: String,
    },
    InvalidStatus {
        command_index: usize,
        reason: String,
    },
    ConflictingStatusReturns {
        index1: usize,
        index2: usize,
        return_name: String,
    },
}

impl std::fmt::Display for RigApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RigApiError::Command(err) => write!(f, "Command error: {err}"),
            RigApiError::CommandNotFound(cmd_type) => write!(f, "{cmd_type} not found"),
            RigApiError::InvalidInit {
                command_index,
                reason,
            } => {
                write!(f, "Invalid init command at index {command_index}: {reason}",)
            }
            RigApiError::InvalidStatus {
                command_index,
                reason,
            } => {
                write!(
                    f,
                    "Invalid status command at index {command_index}: {reason}"
                )
            }
            RigApiError::ConflictingStatusReturns {
                index1,
                index2,
                return_name,
            } => {
                write!(
                    f,
                    "Status command at index {index1} has return value '{return_name}' that conflicts with command at index {index2}"
                )
            }
        }
    }
}

impl std::error::Error for RigApiError {}

impl From<CommandError> for RigApiError {
    fn from(error: CommandError) -> Self {
        RigApiError::Command(error)
    }
}

#[derive(Debug)]
pub struct RigApi {
    init_commands: Vec<Command>,
    commands: HashMap<String, Command>,
    status_commands: Vec<Command>,
}

impl TryFrom<RigFile> for RigApi {
    type Error = RigApiError;
    fn try_from(rig_file: RigFile) -> Result<Self, RigApiError> {
        let init_commands = rig_file
            .init
            .into_iter()
            .map(Command::try_from)
            .collect::<Result<_, _>>()?;

        let commands = rig_file
            .commands
            .into_iter()
            .map(|(name, cmd)| Command::try_from(cmd).map(|command| (name, command)))
            .collect::<Result<_, _>>()?;

        let status_commands = rig_file
            .status
            .into_iter()
            .map(Command::try_from)
            .collect::<Result<_, _>>()?;

        let api = Self {
            init_commands,
            commands,
            status_commands,
        };

        api.validate()?;
        Ok(api)
    }
}

impl RigApi {
    fn validate(&self) -> Result<(), RigApiError> {
        for (i, cmd) in self.init_commands.iter().enumerate() {
            if !cmd.params.is_empty() {
                return Err(RigApiError::InvalidInit {
                    command_index: i,
                    reason: "requires arguments".to_string(),
                });
            }
            if !cmd.returns.is_empty() {
                return Err(RigApiError::InvalidInit {
                    command_index: i,
                    reason: "has return values".to_string(),
                });
            }
        }

        let mut seen_return_values = HashMap::new();
        for (i, cmd) in self.status_commands.iter().enumerate() {
            if !cmd.params.is_empty() {
                return Err(RigApiError::InvalidStatus {
                    command_index: i,
                    reason: "requires arguments".to_string(),
                });
            }

            for return_name in cmd.returns.keys() {
                if let Some(&prev_cmd_idx) = seen_return_values.get(return_name) {
                    return Err(RigApiError::ConflictingStatusReturns {
                        index1: i,
                        index2: prev_cmd_idx,
                        return_name: return_name.clone(),
                    });
                }
                seen_return_values.insert(return_name.clone(), i);
            }
        }

        Ok(())
    }

    pub fn build_init_commands(&self) -> Vec<Result<Vec<u8>, RigApiError>> {
        let empty_args = HashMap::new();
        self.init_commands
            .iter()
            .map(|command| {
                command
                    .build_command(&empty_args)
                    .map_err(RigApiError::from)
            })
            .collect()
    }

    pub fn build_command(
        &self,
        command_name: &str,
        args: &HashMap<String, Value>,
    ) -> Result<Vec<u8>, RigApiError> {
        let command = self.commands.get(command_name).ok_or_else(|| {
            RigApiError::CommandNotFound(CommandType::Named(command_name.to_string()))
        })?;

        command.build_command(args).map_err(RigApiError::from)
    }

    pub fn parse_command_response(
        &self,
        command_name: &str,
        response: &[u8],
    ) -> Result<HashMap<String, Value>, RigApiError> {
        let command = self.commands.get(command_name).ok_or_else(|| {
            RigApiError::CommandNotFound(CommandType::Named(command_name.to_string()))
        })?;

        command.parse_response(response).map_err(RigApiError::from)
    }

    pub fn get_status_commands(&self) -> Vec<Result<Vec<u8>, RigApiError>> {
        let empty_args = HashMap::new();
        self.status_commands
            .iter()
            .map(|command| {
                command
                    .build_command(&empty_args)
                    .map_err(RigApiError::from)
            })
            .collect()
    }

    pub fn parse_status_response(
        &self,
        command_index: usize,
        response: &[u8],
    ) -> Result<HashMap<String, Value>, RigApiError> {
        let command =
            self.status_commands
                .get(command_index)
                .ok_or(RigApiError::CommandNotFound(CommandType::Status(
                    command_index,
                )))?;
        command.parse_response(response).map_err(RigApiError::from)
    }

    pub fn get_init_response_length(
        &self,
        command_index: usize,
    ) -> Result<Option<usize>, RigApiError> {
        let command = self
            .init_commands
            .get(command_index)
            .ok_or(RigApiError::CommandNotFound(CommandType::Init(
                command_index,
            )))?;
        Ok(command.response_length())
    }

    pub fn get_command_response_length(
        &self,
        command_name: &str,
    ) -> Result<Option<usize>, RigApiError> {
        let command = self.commands.get(command_name).ok_or_else(|| {
            RigApiError::CommandNotFound(CommandType::Named(command_name.to_string()))
        })?;
        Ok(command.response_length())
    }

    pub fn get_status_response_length(
        &self,
        command_index: usize,
    ) -> Result<Option<usize>, RigApiError> {
        let command =
            self.status_commands
                .get(command_index)
                .ok_or(RigApiError::CommandNotFound(CommandType::Status(
                    command_index,
                )))?;
        Ok(command.response_length())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig_file::RigCommand;

    fn create_test_command(has_params: bool, has_returns: bool, has_response: bool) -> RigCommand {
        let mut toml_str = String::from("command = \"00\"\n");

        if has_response {
            toml_str.push_str("reply_length = 2\n");
            if has_returns {
                toml_str.push_str("response = \"00.??\"\n");
            } else {
                toml_str.push_str("response = \"00\"\n");
            }
        }

        if has_params {
            toml_str.push_str(
                r#"
                [params.test_param]
                index = 1
                length = 1
                format = "int_bu"
                "#,
            );
            toml_str = toml_str.replace("command = \"00\"", "command = \"00.??\"");
        }

        if has_returns {
            toml_str.push_str(
                r#"
                [returns.test_return]
                index = 1
                length = 1
                format = "int_bu"
                "#,
            );
        }

        toml::from_str(&toml_str).unwrap()
    }

    #[test]
    fn test_rig_api_sanity() -> Result<(), RigApiError> {
        let mut rig_file = RigFile::new();

        rig_file.init.push(create_test_command(false, false, false));

        rig_file.commands.insert(
            "test_cmd".to_string(),
            create_test_command(true, false, true),
        );
        rig_file.status.push(create_test_command(false, true, true));

        let api = RigApi::try_from(rig_file)?;

        let init_cmds = api.build_init_commands();
        assert_eq!(init_cmds.len(), 1);
        assert!(init_cmds[0].is_ok());

        let mut args = HashMap::new();
        args.insert("test_param".to_string(), Value::Int(42));
        let cmd_data = api.build_command("test_cmd", &args)?;
        assert!(!cmd_data.is_empty());

        let status_cmds = api.get_status_commands();
        assert_eq!(status_cmds.len(), 1);
        assert!(status_cmds[0].is_ok());

        Ok(())
    }

    #[test]
    fn test_command_not_found() {
        let rig_file = RigFile::new();
        let api = RigApi::try_from(rig_file).unwrap();

        match api.build_command("nonexistent", &HashMap::new()) {
            Err(RigApiError::CommandNotFound(CommandType::Named(name))) => {
                assert_eq!(name, "nonexistent");
            }
            _ => panic!("Expected CommandNotFound error"),
        }

        match api.get_init_response_length(0) {
            Err(RigApiError::CommandNotFound(CommandType::Init(idx))) => {
                assert_eq!(idx, 0);
            }
            _ => panic!("Expected CommandNotFound error"),
        }

        match api.get_status_response_length(0) {
            Err(RigApiError::CommandNotFound(CommandType::Status(idx))) => {
                assert_eq!(idx, 0);
            }
            _ => panic!("Expected CommandNotFound error"),
        }
    }

    #[test]
    fn test_invalid_init_command() {
        let mut rig_file = RigFile::new();

        rig_file.init.push(create_test_command(true, false, false));

        match RigApi::try_from(rig_file) {
            Err(RigApiError::InvalidInit {
                command_index,
                reason,
            }) => {
                assert_eq!(command_index, 0);
                assert_eq!(reason, "requires arguments");
            }
            _ => panic!("Expected InvalidInit error"),
        }

        let mut rig_file = RigFile::new();
        rig_file.init.push(create_test_command(false, true, true));

        match RigApi::try_from(rig_file) {
            Err(RigApiError::InvalidInit {
                command_index,
                reason,
            }) => {
                assert_eq!(command_index, 0);
                assert_eq!(reason, "has return values");
            }
            _ => panic!("Expected InvalidInit error"),
        }
    }

    #[test]
    fn test_invalid_status_command() {
        let mut rig_file = RigFile::new();

        rig_file.status.push(create_test_command(true, true, true));

        match RigApi::try_from(rig_file) {
            Err(RigApiError::InvalidStatus {
                command_index,
                reason,
            }) => {
                assert_eq!(command_index, 0);
                assert_eq!(reason, "requires arguments");
            }
            _ => panic!("Expected InvalidStatus error"),
        }
    }

    #[test]
    fn test_conflicting_status_returns() {
        let mut rig_file = RigFile::new();

        rig_file.status.push(create_test_command(false, true, true));
        rig_file.status.push(create_test_command(false, true, true));

        match RigApi::try_from(rig_file) {
            Err(RigApiError::ConflictingStatusReturns {
                index1,
                index2,
                return_name,
            }) => {
                assert_eq!(index1, 1);
                assert_eq!(index2, 0);
                assert_eq!(return_name, "test_return");
            }
            _ => panic!("Expected ConflictingStatusReturns error"),
        }
    }
}
