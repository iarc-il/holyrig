mod error;
mod handler;
mod methods;
mod server;
mod types;

pub use error::RpcError;
pub use handler::RpcHandler;
pub use methods::RigRpcHandler;
pub use server::JsonRpcServer;
pub use types::{Id, Notification, Request, Response, Version};

pub const VERSION: &str = "2.0";
