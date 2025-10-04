mod error;
mod handler;
mod server;
mod types;

pub use error::RpcError;
pub use handler::RigRpcHandler;
pub use server::JsonRpcServer;
pub use types::{Notification, Request, Response};

pub const VERSION: &str = "2.0";
