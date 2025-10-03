use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::RpcError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    pub id: String,
}

impl Request {
    pub fn get_rig_id(&self) -> Option<usize> {
        if let Some(Value::Object(params)) = &self.params
            && let Some(Value::Number(id)) = params.get("rig_id")
        {
            Some(id.as_u64()? as usize)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: String,
}

impl Response {
    pub fn build_error(id: String, error: RpcError) -> Response {
        Response {
            jsonrpc: super::VERSION.into(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
}
