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
        // Check init commands
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

        // Check status commands
        let mut seen_return_values = HashMap::new();
        for (i, cmd) in self.status_commands.iter().enumerate() {
            if !cmd.params.is_empty() {
                return Err(RigApiError::InvalidStatus {
                    command_index: i,
                    reason: "requires arguments".to_string(),
                });
            }

            // Check for conflicting return value names
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
