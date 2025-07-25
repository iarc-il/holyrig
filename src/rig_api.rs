use std::collections::HashMap;

use crate::{
    commands::{Command, CommandError, Value},
    rig_file::RigFile,
    schema::{self, ValueType},
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
    InvalidEnumValue {
        enum_name: String,
        value: i64,
    },
    UnknownParam {
        command_name: String,
        param_name: String,
    },
    BuildValueFailed {
        error: String,
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
            RigApiError::InvalidEnumValue { enum_name, value } => {
                write!(f, "Invalid enum value {value} of enum '{enum_name}'")
            }
            RigApiError::UnknownParam {
                command_name,
                param_name,
            } => write!(f, "Unknown param {param_name} in command {command_name}"),
            RigApiError::BuildValueFailed { error } => write!(f, "Failed to build value: {error}"),
        }
    }
}

impl std::error::Error for RigApiError {}

impl From<CommandError> for RigApiError {
    fn from(error: CommandError) -> Self {
        RigApiError::Command(error)
    }
}

#[derive(Debug, Clone)]
pub struct RigApi {
    init_commands: Vec<Command>,
    commands: HashMap<String, Command>,
    status_commands: Vec<Command>,
    enum_mappings: HashMap<String, HashMap<String, i32>>,
    reverse_enum_mappings: HashMap<String, HashMap<i32, String>>,
    command_param_types: HashMap<String, HashMap<String, ValueType>>,
    command_return_types: HashMap<String, HashMap<String, ValueType>>,
}

impl TryFrom<(RigFile, schema::Schema)> for RigApi {
    type Error = RigApiError;
    fn try_from((rig_file, schema): (RigFile, schema::Schema)) -> Result<Self, RigApiError> {
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

        let mut enum_mappings = HashMap::new();
        let mut reverse_enum_mappings = HashMap::new();

        for (enum_name, mapping) in rig_file.enums {
            let mut value_map = HashMap::new();
            let mut reverse_map = HashMap::new();
            for (member, value) in mapping.values {
                value_map.insert(member.clone(), value);
                reverse_map.insert(value, member);
            }
            enum_mappings.insert(enum_name.clone(), value_map);
            reverse_enum_mappings.insert(enum_name, reverse_map);
        }

        let mut command_param_types = HashMap::new();
        let mut command_return_types = HashMap::new();

        for (cmd_name, cmd) in &schema.commands {
            command_param_types.insert(cmd_name.clone(), cmd.params.iter().cloned().collect());
            command_return_types.insert(cmd_name.clone(), cmd.returns.iter().cloned().collect());
        }

        let api = Self {
            init_commands,
            commands,
            status_commands,
            enum_mappings,
            reverse_enum_mappings,
            command_param_types,
            command_return_types,
        };

        api.validate()?;

        Ok(api)
    }
}

impl RigApi {
    pub fn get_enum_value(&self, enum_name: &str, member: &str) -> Option<i32> {
        self.enum_mappings
            .get(enum_name)
            .and_then(|map| map.get(member))
            .copied()
    }

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

    pub fn build_init_commands(&self) -> Result<Vec<Vec<u8>>, RigApiError> {
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

        let mut converted_args = HashMap::new();
        for (name, value) in args {
            match value {
                Value::Enum(member) => {
                    if let Some(enum_name) = self.get_enum_type_for_param(command_name, name) {
                        let int_value =
                            self.get_enum_value(&enum_name, member).ok_or_else(|| {
                                RigApiError::Command(CommandError::InvalidArgumentValue(format!(
                                    "Invalid enum value '{member}' for enum '{enum_name}'",
                                )))
                            })?;
                        converted_args.insert(name.clone(), Value::Int(int_value as i64));
                    } else {
                        return Err(RigApiError::Command(CommandError::InvalidArgumentValue(
                            format!("Parameter '{name}' is not an enum type"),
                        )));
                    }
                }
                _ => {
                    converted_args.insert(name.clone(), value.clone());
                }
            }
        }

        command
            .build_command(&converted_args)
            .map_err(RigApiError::from)
    }

    pub fn parse_command_response(
        &self,
        command_name: &str,
        response: &[u8],
    ) -> Result<HashMap<String, Value>, RigApiError> {
        let command = self.commands.get(command_name).ok_or_else(|| {
            RigApiError::CommandNotFound(CommandType::Named(command_name.to_string()))
        })?;

        let raw_values = command
            .parse_response(response)
            .map_err(RigApiError::from)?;

        let mut converted_values = HashMap::new();
        for (name, value) in raw_values {
            if let Some(return_types) = self.command_return_types.get(command_name) {
                if let Some(type_name) = return_types.get(&name) {
                    match type_name {
                        ValueType::Int => {
                            converted_values.insert(name, Value::Int(value));
                        }
                        ValueType::Bool => {
                            let value = value != 0;
                            converted_values.insert(name, Value::Bool(value));
                        }
                        ValueType::Enum(enum_name) => {
                            if let Some(enum_map) = self.reverse_enum_mappings.get(enum_name) {
                                if let Some(enum_value) = enum_map.get(&(value as i32)) {
                                    converted_values.insert(name, Value::Enum(enum_value.clone()));
                                    continue;
                                } else {
                                    return Err(RigApiError::InvalidEnumValue {
                                        enum_name: enum_name.clone(),
                                        value,
                                    });
                                }
                            }
                        }
                    }
                }
            } else {
                // Already checked at the begining of the function
                panic!();
            }
        }

        Ok(converted_values)
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

    pub fn parse_param_values(
        &self,
        command_name: &str,
        params: HashMap<String, String>,
    ) -> Result<HashMap<String, Value>, RigApiError> {
        params
            .into_iter()
            .map(|(param_name, value)| {
                let value_type = self
                    .get_command_param_type(command_name, &param_name)
                    .ok_or(RigApiError::UnknownParam {
                        command_name: command_name.to_string(),
                        param_name: param_name.clone(),
                    })?;
                let parsed_value = value_type.build_value(&value).map_err(|err| {
                    RigApiError::BuildValueFailed {
                        error: err.to_string(),
                    }
                })?;
                Ok((param_name, parsed_value))
            })
            .collect::<Result<_, _>>()
    }

    fn get_command_param_type(&self, command_name: &str, param_name: &str) -> Option<&ValueType> {
        self.command_param_types
            .get(command_name)
            .and_then(|params| params.get(param_name))
    }

    fn get_enum_type_for_param(&self, command_name: &str, param_name: &str) -> Option<String> {
        self.get_command_param_type(command_name, param_name)
            .and_then(|type_name| {
                if let ValueType::Enum(enum_name) = type_name
                    && self.enum_mappings.contains_key(enum_name)
                {
                    Some(enum_name.clone())
                } else {
                    None
                }
            })
    }

    fn get_enum_type_for_return(&self, command_name: &str, return_name: &str) -> Option<String> {
        self.command_return_types
            .get(command_name)
            .and_then(|returns| returns.get(return_name))
            .and_then(|type_name| {
                if let ValueType::Enum(enum_name) = type_name
                    && self.enum_mappings.contains_key(enum_name)
                {
                    Some(enum_name.clone())
                } else {
                    None
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rig_file::RigCommand,
        schema::{Enum, Schema},
    };

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

        let api = RigApi::try_from((rig_file, schema::Schema::new()))?;

        let init_cmds = api.build_init_commands()?;
        assert_eq!(init_cmds.len(), 1);

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
        let api = RigApi::try_from((rig_file, schema::Schema::new())).unwrap();

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

        match RigApi::try_from((rig_file, schema::Schema::new())) {
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

        match RigApi::try_from((rig_file, schema::Schema::new())) {
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

        match RigApi::try_from((rig_file, schema::Schema::new())) {
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

        match RigApi::try_from((rig_file, schema::Schema::new())) {
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

    fn create_test_schema() -> Schema {
        let mut schema = Schema::new();

        let mode_enum = Enum {
            members: vec!["LSB".to_string(), "USB".to_string(), "CW".to_string()],
        };
        schema.enums.insert("Mode".to_string(), mode_enum);

        let mode_value_type = ValueType::Enum("Mode".to_string());
        let set_mode_cmd = schema::Command {
            params: vec![("mode".to_string(), mode_value_type.clone())],
            returns: vec![],
        };
        let get_mode_cmd = schema::Command {
            params: vec![],
            returns: vec![("mode".to_string(), mode_value_type)],
        };
        schema.commands.insert("set_mode".to_string(), set_mode_cmd);
        schema.commands.insert("get_mode".to_string(), get_mode_cmd);

        schema
    }

    fn create_test_rig_file() -> RigFile {
        let mut rig_file = RigFile::new();

        let mode_mapping = crate::rig_file::EnumMapping {
            values: vec![
                ("LSB".to_string(), 0),
                ("USB".to_string(), 1),
                ("CW".to_string(), 2),
            ],
        };
        rig_file.enums.insert("Mode".to_string(), mode_mapping);

        let set_mode_cmd = crate::rig_file::RigCommand {
            command: "AA??".to_string(),
            response: None,
            reply_length: None,
            reply_end: None,
            params: {
                let mut params = HashMap::new();
                params.insert(
                    "mode".to_string(),
                    crate::rig_file::RigBinaryParam {
                        index: 1,
                        length: 1,
                        format: crate::data_format::DataFormat::IntLu,
                        multiply: 1.0,
                        add: 0.0,
                    },
                );
                params
            },
            returns: HashMap::new(),
        };
        let get_mode_cmd = crate::rig_file::RigCommand {
            command: "AABB".to_string(),
            response: Some("AA??".to_string()),
            reply_length: Some(2),
            reply_end: None,
            params: HashMap::new(),
            returns: {
                let mut returns = HashMap::new();
                returns.insert(
                    "mode".to_string(),
                    crate::rig_file::RigBinaryParam {
                        index: 1,
                        length: 1,
                        format: crate::data_format::DataFormat::IntLu,
                        multiply: 1.0,
                        add: 0.0,
                    },
                );
                returns
            },
        };
        rig_file
            .commands
            .insert("set_mode".to_string(), set_mode_cmd);
        rig_file
            .commands
            .insert("get_mode".to_string(), get_mode_cmd);

        rig_file
    }

    #[test]
    fn test_enum_value_conversion() {
        let schema = create_test_schema();
        let rig_file = create_test_rig_file();

        let rig_api = RigApi::try_from((rig_file, schema)).unwrap();

        assert_eq!(rig_api.get_enum_value("Mode", "LSB"), Some(0));
        assert_eq!(rig_api.get_enum_value("Mode", "USB"), Some(1));
        assert_eq!(rig_api.get_enum_value("Mode", "CW"), Some(2));
        assert_eq!(rig_api.get_enum_value("Mode", "AM"), None);
        assert_eq!(rig_api.get_enum_value("NonExistentEnum", "LSB"), None);
    }

    #[test]
    fn test_build_command_with_enum() {
        let schema = create_test_schema();
        let rig_file = create_test_rig_file();

        let rig_api = RigApi::try_from((rig_file, schema)).unwrap();

        let mut args = HashMap::new();
        args.insert("mode".to_string(), Value::Enum("USB".to_string()));
        let cmd = rig_api.build_command("set_mode", &args).unwrap();
        assert_eq!(cmd, b"\xAA\x01");

        let mut args = HashMap::new();
        args.insert("mode".to_string(), Value::Enum("AM".to_string()));
        assert!(rig_api.build_command("set_mode", &args).is_err());
    }

    #[test]
    fn test_parse_response_with_enum() {
        let schema = create_test_schema();
        let rig_file = create_test_rig_file();

        let rig_api = RigApi::try_from((rig_file, schema)).unwrap();

        let response = b"\xaa\x02";
        let values = rig_api
            .parse_command_response("get_mode", response)
            .unwrap();

        match values.get("mode") {
            Some(Value::Enum(mode)) => assert_eq!(mode, "CW"),
            other => panic!("Expected Enum value for mode, got {other:?}"),
        }

        let response = b"\xaa\x03";
        let parsed_response = rig_api.parse_command_response("get_mode", response);
        assert!(parsed_response.is_err());
    }
}
