use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::RpcError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum Id {
    #[default]
    Null,
    Number(i64),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    pub id: Id,
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
    pub id: Id,
}

impl Response {
    pub fn build_error(mut error: RpcError) -> Self {
        let id = std::mem::take(&mut error.id);
        Response {
            jsonrpc: super::VERSION.into(),
            result: None,
            error: Some(error),
            id,
        }
    }

    pub fn build_success(id: Id) -> Self {
        Response {
            jsonrpc: super::VERSION.into(),
            result: Some(json!({"result": "success"})),
            error: None,
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
