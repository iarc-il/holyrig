use anyhow::Result;
use serde_json::Value;

use super::{Request, Response, RpcError};

#[async_trait::async_trait]
pub trait RpcHandler: Send + Sync {
    async fn handle_request(&self, request: Request) -> Result<Response>;
    fn supported_methods(&self) -> &[&str];
    fn create_response(&self, id: String, result: Value) -> Response {
        Response {
            jsonrpc: super::Version(super::VERSION.to_string()),
            result: Some(result),
            error: None,
            id,
        }
    }
    fn create_error_response(&self, id: String, error: RpcError) -> Response {
        Response {
            jsonrpc: super::Version(super::VERSION.to_string()),
            result: None,
            error: Some(error),
            id,
        }
    }
}
