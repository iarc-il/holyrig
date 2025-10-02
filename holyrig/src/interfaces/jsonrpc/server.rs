use anyhow::Result;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, mpsc};

use super::{Notification, RigRpcHandler};
use crate::interfaces::jsonrpc::{self, Request, Response};
use crate::resources::Resources;
use crate::runtime::Value;
use crate::serial::manager::{CommandResponse, ManagerCommand, ManagerMessage};

pub struct JsonRpcServer {
    bind_address: String,
    port: u16,
    handler: Arc<RigRpcHandler>,
    resources: Arc<Resources>,
    command_tx: mpsc::Sender<ManagerCommand>,
    manager_rx: broadcast::Receiver<ManagerMessage>,
}

impl JsonRpcServer {
    pub fn new(
        bind_address: &str,
        port: u16,
        resources: Arc<Resources>,
        command_tx: mpsc::Sender<ManagerCommand>,
        manager_rx: broadcast::Receiver<ManagerMessage>,
    ) -> Result<Self> {
        let handler = Arc::new(RigRpcHandler::new(
            resources.schema.clone(),
            HashSet::new(),
            HashSet::new(),
            command_tx.clone(),
        ));

        Ok(Self {
            bind_address: bind_address.to_string(),
            port,
            handler,
            resources,
            command_tx,
            manager_rx,
        })
    }

    pub async fn run(mut self) -> Result<()> {
        let addr = format!("{}:{}", self.bind_address, self.port);

        tokio::spawn(async move {
            let socket = UdpSocket::bind(&addr).await.unwrap();
            println!("JSON-RPC UDP server listening on {}", addr);

            let mut buf = vec![0u8; 2048];
            loop {
                tokio::select! {
                    received = socket.recv_from(&mut buf) => {
                         match received {
                            Ok((len, src_addr)) => {
                                if let Err(e) =
                                    self.handle_packet(&socket, &buf[..len], src_addr).await
                                {
                                    eprintln!("Error handling UDP datagram: {}", e);
                                }
                            },
                            Err(err) => {
                                eprintln!("Failed to receive data: {err}");
                            }
                        }
                    }
                    message = self.manager_rx.recv() => {
                        let message = message.unwrap();
                        if let Err(err) = self.handle_manager_message(message).await {
                            eprintln!("Error handling manager message: {err}");
                        }
                    }
                }
            }
        });
        Ok(())
    }

    async fn handle_packet(
        &self,
        socket: &UdpSocket,
        data: &[u8],
        src_addr: std::net::SocketAddr,
    ) -> Result<()> {
        match serde_json::from_slice::<Request>(data) {
            Ok(request) => {
                let response = self.handler.handle_request(request).await?;
                let response_data = serde_json::to_vec(&response)?;
                socket.send_to(&response_data, src_addr).await?;
            },
            Err(err) => {
                let error_response = Response {
                    jsonrpc: super::VERSION.into(),
                    result: None,
                    error: Some(jsonrpc::RpcError::new(-32700, format!("Parse error: {err}"))),
                    id: String::new(),
                };
                let error_data = serde_json::to_vec(&error_response)?;
                socket.send_to(&error_data, src_addr).await?;
            },
        }

        Ok(())
    }

    async fn handle_manager_message(&self, msg: ManagerMessage) -> Result<()> {
        match msg {
            ManagerMessage::DeviceConnected { device_id } => {
                self.handle_device_connected(device_id).await?;
            }
            ManagerMessage::DeviceDisconnected { device_id } => {
                self.handle_device_disconnected(device_id).await?;
            }
            ManagerMessage::StatusUpdate { device_id, values } => {
                self.handle_status_update(device_id, values).await?;
            }
            ManagerMessage::CommandResponse {
                device_id,
                command_name,
                response,
            } => {
                self.handle_command_response(device_id, command_name, response)
                    .await?;
            }
        }
        Ok(())
    }

    async fn handle_device_connected(&self, device_id: usize) -> Result<()> {
        let notification = Notification {
            jsonrpc: super::VERSION.into(),
            method: "device_connected".to_string(),
            params: json!({
                "device_id": device_id,
            }),
        };
        // self.transport.broadcast_notification(notification).await?;
        Ok(())
    }

    async fn handle_device_disconnected(&self, device_id: usize) -> Result<()> {
        // Notify clients
        let notification = Notification {
            jsonrpc: super::VERSION.into(),
            method: "device_disconnected".to_string(),
            params: json!({
                "device_id": device_id,
            }),
        };
        // self.transport.broadcast_notification(notification).await?;
        Ok(())
    }

    async fn handle_status_update(
        &self,
        device_id: usize,
        values: HashMap<String, Value>,
    ) -> Result<()> {
        let values: HashMap<_, _> = values
            .into_iter()
            .map(|(k, v)| (k, serde_json::Value::from(v)))
            .collect();

        let notification = Notification {
            jsonrpc: super::VERSION.into(),
            method: "status_update".to_string(),
            params: json!({
                "device_id": device_id,
                "values": values,
            }),
        };
        // self.transport.broadcast_notification(notification).await?;
        Ok(())
    }

    async fn handle_command_response(
        &self,
        device_id: usize,
        command_name: String,
        command_response: CommandResponse,
    ) -> Result<()> {
        let response = match &command_response {
            CommandResponse::Success(msg) => json!(msg
                .iter()
                .map(|(k, v)| (k, serde_json::Value::from(v)))
                .collect::<HashMap<_, _>>()),
            CommandResponse::Error(err) => json!(err),
        };

        let notification = Notification {
            jsonrpc: super::VERSION.into(),
            method: "command_response".to_string(),
            params: json!({
                "device_id": device_id,
                "command": command_name,
                "success": matches!(command_response, CommandResponse::Success(_)),
                "response": response,
            }),
        };
        // self.transport.broadcast_notification(notification).await?;
        Ok(())
    }
}
