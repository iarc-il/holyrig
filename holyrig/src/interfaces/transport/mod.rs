use anyhow::Result;
use crate::interfaces::jsonrpc::Notification;

pub mod udp;

#[derive(Debug, Clone)]
pub struct Client {
    pub id: String,
    pub addr: std::net::SocketAddr,
    pub subscriptions: Vec<String>,
}

#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn send_notification(
        &self,
        notification: Notification,
        client_ids: &[String],
    ) -> Result<()>;
    async fn broadcast_notification(&self, notification: Notification) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub bind_address: String,
    pub port: u16,
}
