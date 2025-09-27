use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;

use super::{Client, Transport, TransportConfig};
use crate::interfaces::jsonrpc::{self, Notification, Request, Response, RpcHandler};

pub struct UdpTransport {
    config: TransportConfig,
    handler: Arc<dyn RpcHandler>,
    socket: Option<Arc<UdpSocket>>,
    clients: Arc<RwLock<HashMap<String, Client>>>,
}

impl UdpTransport {
    pub fn new(config: TransportConfig, handler: Arc<dyn RpcHandler>) -> Result<Self> {
        Ok(Self {
            config,
            handler,
            socket: None,
            clients: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn handle_datagram(
        handler: Arc<dyn RpcHandler>,
        _clients: Arc<RwLock<HashMap<String, Client>>>,
        socket: Arc<UdpSocket>,
        data: &[u8],
        src_addr: std::net::SocketAddr,
    ) -> Result<()> {
        if let Ok(request) = serde_json::from_slice::<Request>(data) {
            let response = handler.handle_request(request).await?;
            let response_data = serde_json::to_vec(&response)?;
            socket.send_to(&response_data, src_addr).await?;
        } else {
            let error_response = Response {
                jsonrpc: jsonrpc::Version(String::from("2.0")),
                result: None,
                error: Some(jsonrpc::RpcError::new(
                    -32700,
                    "Parse error",
                )),
                id: jsonrpc::Id::Null,
            };
            let error_data = serde_json::to_vec(&error_response)?;
            socket.send_to(&error_data, src_addr).await?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Transport for UdpTransport {
    async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.bind_address, self.config.port);
        let socket = Arc::new(UdpSocket::bind(&addr).await?);
        println!("JSON-RPC UDP server listening on {}", addr);

        let handler = Arc::clone(&self.handler);
        let clients = Arc::clone(&self.clients);
        let socket_clone = Arc::clone(&socket);

        let mut buf = vec![0u8; 2048];
        loop {
            let (len, src_addr) = socket.recv_from(&mut buf).await?;
            let data = buf[..len].to_vec();

            let handler = Arc::clone(&handler);
            let clients = Arc::clone(&clients);
            let socket = Arc::clone(&socket_clone);

            tokio::spawn(async move {
                if let Err(e) =
                    Self::handle_datagram(handler, clients, socket, &data, src_addr).await
                {
                    eprintln!("Error handling UDP datagram: {}", e);
                }
            });
        }
    }

    async fn stop(&self) -> Result<()> {
        self.clients.write().await.clear();
        Ok(())
    }

    async fn send_notification(
        &self,
        notification: Notification,
        client_ids: &[String],
    ) -> Result<()> {
        let notification_data = serde_json::to_vec(&notification)?;
        let clients = self.clients.read().await;

        for client_id in client_ids {
            if let Some(client) = clients.get(client_id) {
                self.socket
                    .as_ref()
                    .unwrap()
                    .send_to(&notification_data, client.addr)
                    .await?;
            }
        }

        Ok(())
    }

    async fn broadcast_notification(&self, notification: Notification) -> Result<()> {
        let notification_data = serde_json::to_vec(&notification)?;
        let clients = self.clients.read().await;

        for client in clients.values() {
            self.socket
                .as_ref()
                .unwrap()
                .send_to(&notification_data, client.addr)
                .await?;
        }

        Ok(())
    }
}
