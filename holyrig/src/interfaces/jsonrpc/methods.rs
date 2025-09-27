use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc::Sender;

use super::{Request, Response, RpcError, RpcHandler};
use crate::runtime::SchemaFile;
use crate::serial::manager::ManagerCommand;

pub struct RigRpcHandler {
    schema: SchemaFile,
    implemented_commands: HashSet<String>,
    implemented_status: HashSet<String>,
    command_sender: Sender<ManagerCommand>,
}

impl RigRpcHandler {
    pub fn new(
        schema: SchemaFile,
        implemented_commands: HashSet<String>,
        implemented_status: HashSet<String>,
        command_sender: Sender<ManagerCommand>,
    ) -> Self {
        Self {
            schema,
            implemented_commands,
            implemented_status,
            command_sender,
        }
    }

    fn get_capabilities(&self) -> Result<Value> {
        let mut capabilities = serde_json::Map::new();

        let mut commands = serde_json::Map::new();
        for cmd_name in &self.implemented_commands {
            let params = self
                .schema
                .commands
                .get(cmd_name)
                .expect("Implemented command should be in the schema");

            let mut cmd_info = serde_json::Map::new();
            let mut parameters = serde_json::Map::new();

            for param in params {
                parameters.insert(
                    param.name.clone(),
                    Value::String(param.param_type.to_string()),
                );
            }

            cmd_info.insert("parameters".to_string(), Value::Object(parameters));
            commands.insert(cmd_name.clone(), Value::Object(cmd_info));
        }
        capabilities.insert("commands".to_string(), Value::Object(commands));

        let mut status_fields = serde_json::Map::new();
        for field_name in &self.implemented_status {
            let field_type = self
                .schema
                .status
                .get(field_name)
                .expect("Implemented status field should be in the schema");
            status_fields.insert(field_name.clone(), Value::String(field_type.to_string()));
        }
        capabilities.insert("status_fields".to_string(), Value::Object(status_fields));

        Ok(Value::Object(capabilities))
    }

    async fn execute_command(
        &self,
        command: String,
        params: HashMap<String, Value>,
    ) -> Result<Value> {
        let command_params = self
            .schema
            .commands
            .get(&command)
            .ok_or_else(|| anyhow!(RpcError::method_not_found()))?;

        if !self.implemented_commands.contains(&command) {
            return Err(anyhow!(RpcError::new(
                -32001,
                format!("Command '{}' is not implemented by this rig", command)
            )));
        }

        let mut string_params = HashMap::new();
        for param in command_params {
            let value = params.get(&param.name).ok_or_else(|| {
                anyhow!(RpcError::invalid_command_params(format!(
                    "Missing parameter: {}",
                    param.name
                )))
            })?;

            string_params.insert(param.name.clone(), value.to_string());
        }

        self.command_sender
            .send(ManagerCommand::ExecuteCommand {
                device_id: 0, // TODO: Handle multiple devices
                command_name: command,
                params: string_params,
            })
            .await?;

        Ok(Value::Object(serde_json::Map::new()))
    }
}

#[async_trait::async_trait]
impl RpcHandler for RigRpcHandler {
    async fn handle_request(&self, request: Request) -> Result<Response> {
        let response = match request.method.as_str() {
            "get_capabilities" => {
                let result = self.get_capabilities()?;
                self.create_response(request.id, result)
            }
            "execute_command" => {
                let params = request
                    .params
                    .ok_or_else(|| anyhow!(RpcError::invalid_params()))?;
                let params_map = params
                    .as_object()
                    .ok_or_else(|| anyhow!(RpcError::invalid_params()))?;

                let command = params_map
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!(RpcError::invalid_params()))?
                    .to_string();

                let parameters = params_map
                    .get("parameters")
                    .and_then(|v| v.as_object())
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default();

                match self.execute_command(command, parameters).await {
                    Ok(result) => self.create_response(request.id, result),
                    Err(err) => {
                        if let Some(rpc_err) = err.downcast_ref::<RpcError>() {
                            self.create_error_response(request.id, rpc_err.clone())
                        } else {
                            self.create_error_response(
                                request.id,
                                RpcError::rig_communication_error(err.to_string()),
                            )
                        }
                    }
                }
            }
            _ => self.create_error_response(request.id, RpcError::method_not_found()),
        };

        Ok(response)
    }

    fn supported_methods(&self) -> &[&str] {
        &["get_capabilities", "execute_command"]
    }
}
