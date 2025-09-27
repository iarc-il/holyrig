mod error;
mod handler;
mod types;

pub use error::RpcError;
pub use handler::RpcHandler;
pub use types::{Id, Notification, Request, Response, Version};

pub const VERSION: &str = "2.0";
