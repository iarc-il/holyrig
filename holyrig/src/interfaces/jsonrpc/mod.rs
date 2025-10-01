mod error;
mod methods;
mod server;
mod types;

pub use error::RpcError;
pub use methods::RigRpcHandler;
pub use server::JsonRpcServer;
pub use types::{Notification, Request, Response};

pub const VERSION: &str = "2.0";
