use std::collections::HashMap;

use crate::{
    commands::{Command, CommandError, Value},
    rig_file::RigFile,
};

#[derive(Debug)]
pub struct RigApi {
    init_commands: Vec<Command>,
    commands: HashMap<String, Command>,
    status_commands: Vec<Command>,
}

impl TryFrom<RigFile> for RigApi {
    type Error = CommandError;
    fn try_from(rig_file: RigFile) -> Result<Self, CommandError> {
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

        Ok(Self {
            init_commands,
            commands,
            status_commands,
        })
    }
}

impl RigApi {
    pub fn build_init_commands(&self) -> Vec<Vec<u8>> {
        let empty_args = HashMap::new();
        self.init_commands
            .iter()
            .map(|command| command.build_command(&empty_args).unwrap())
            .collect()
    }

    pub fn build_command(
        &self,
        command_name: &str,
        args: &HashMap<String, Value>,
    ) -> Result<Vec<u8>, CommandError> {
        // TODO return error if command is missing
        let command = self.commands.get(command_name).unwrap();

        command.build_command(args)
    }

    pub fn parse_command_response(
        &self,
        command_name: &str,
        response: &[u8],
    ) -> Result<HashMap<String, Value>, CommandError> {
        let command = self.commands.get(command_name).unwrap();

        command.parse_response(response)
    }

    pub fn get_status_commands(&self) -> Vec<Vec<u8>> {
        let empty_args = HashMap::new();
        self.status_commands
            .iter()
            .map(|command| command.build_command(&empty_args).unwrap())
            .collect()
    }

    pub fn parse_status_response(
        &self,
        command_index: usize,
        response: &[u8],
    ) -> Result<HashMap<String, Value>, CommandError> {
        let command = self.status_commands.get(command_index).unwrap();
        command.parse_response(response)
    }

    pub fn get_init_response_length(&self, command_index: usize) -> Option<usize> {
        self.init_commands
            .get(command_index)
            .and_then(|cmd| cmd.response_length())
    }

    pub fn get_command_response_length(&self, command_name: &str) -> Option<usize> {
        self.commands
            .get(command_name)
            .and_then(|cmd| cmd.response_length())
    }

    pub fn get_status_response_length(&self, command_index: usize) -> Option<usize> {
        self.status_commands
            .get(command_index)
            .and_then(|cmd| cmd.response_length())
    }
}
