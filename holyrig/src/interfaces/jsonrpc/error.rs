use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    // Standard JSON-RPC error codes
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    // Custom error codes for rig operations
    pub const RIG_COMMUNICATION_ERROR: i32 = -32000;
    pub const INVALID_COMMAND_PARAMS: i32 = -32001;
    pub const SUBSCRIPTION_ERROR: i32 = -32002;

    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(code: i32, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn parse_error() -> Self {
        Self::new(Self::PARSE_ERROR, "Parse error")
    }

    pub fn invalid_request() -> Self {
        Self::new(Self::INVALID_REQUEST, "Invalid Request")
    }

    pub fn method_not_found() -> Self {
        Self::new(Self::METHOD_NOT_FOUND, "Method not found")
    }

    pub fn invalid_params() -> Self {
        Self::new(Self::INVALID_PARAMS, "Invalid params")
    }

    pub fn internal_error() -> Self {
        Self::new(Self::INTERNAL_ERROR, "Internal error")
    }

    pub fn rig_communication_error(msg: impl Into<String>) -> Self {
        Self::new(Self::RIG_COMMUNICATION_ERROR, msg)
    }

    pub fn invalid_command_params(msg: impl Into<String>) -> Self {
        Self::new(Self::INVALID_COMMAND_PARAMS, msg)
    }

    pub fn subscription_error(msg: impl Into<String>) -> Self {
        Self::new(Self::SUBSCRIPTION_ERROR, msg)
    }
}
