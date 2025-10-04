use anyhow::Result;
use parking_lot::RwLock;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, mpsc};

use super::{Notification, RigRpcHandler};
use crate::interfaces::jsonrpc::{self, Request, Response};
use crate::resources::Resources;
use crate::runtime::Value;
use crate::serial::manager::{ManagerCommand, ManagerMessage};

pub struct JsonRpcServer {
    bind_address: String,
    port: u16,
    handlers: Arc<HashMap<String, RigRpcHandler>>,
    rigs_state: Arc<RwLock<HashMap<usize, (String, bool)>>>,
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
        let handlers = resources
            .rigs
            .iter()
            .map(|(rig_name, interpreter)| {
                let rig_file = interpreter.rig_file();
                let schema = resources.schemas.get(&rig_file.impl_block.schema).unwrap();
                let handler = RigRpcHandler::new(rig_file, schema, command_tx.clone());
                (rig_name.clone(), handler)
            })
            .collect();

        Ok(Self {
            bind_address: bind_address.to_string(),
            port,
            handlers: Arc::new(handlers),
            rigs_state: Arc::new(RwLock::new(HashMap::new())),
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
        let response = match serde_json::from_slice::<Request>(data) {
            Ok(request) => match request.method.as_str() {
                "list_rigs" => {
                    let rigs = serde_json::Value::Object(
                        self.rigs_state
                            .read()
                            .iter()
                            .map(|(device_id, (_, is_connected))| {
                                (
                                    device_id.to_string(),
                                    serde_json::Value::Bool(*is_connected),
                                )
                            })
                            .collect(),
                    );
                    Response {
                        jsonrpc: super::VERSION.into(),
                        result: Some(rigs),
                        error: None,
                        id: request.id,
                    }
                }
                _ => {
                    if let Some(id) = request.get_rig_id() {
                        let handler = {
                            let rigs = self.rigs_state.read();
                            let (rig_model, _) = rigs.get(&id).unwrap();
                            self.handlers.get(rig_model)
                        };
                        if let Some(handler) = handler {
                            handler.handle_request(&request, id).await?
                        } else {
                            Response::build_error(request.id, jsonrpc::RpcError::unknown_rig_id(id))
                        }
                    } else {
                        Response::build_error(request.id, jsonrpc::RpcError::missing_rig_id())
                    }
                }
            },
            Err(err) => Response::build_error(String::new(), jsonrpc::RpcError::parse_error(&err)),
        };

        let error_data = serde_json::to_vec(&response)?;
        socket.send_to(&error_data, src_addr).await?;

        Ok(())
    }

    async fn handle_manager_message(&self, message: ManagerMessage) -> Result<()> {
        match message {
            ManagerMessage::InitialState { rigs } => {
                *self.rigs_state.write() = rigs
                    .iter()
                    .map(|(device_id, rig_model)| (*device_id, (rig_model.clone(), false)))
                    .collect();
            }
            ManagerMessage::DeviceConnected {
                device_id,
                rig_model,
            } => {
                self.handle_device_connected(device_id, rig_model).await?;
            }
            ManagerMessage::DeviceDisconnected { device_id } => {
                self.handle_device_disconnected(device_id).await?;
            }
            ManagerMessage::StatusUpdate { device_id, values } => {
                self.handle_status_update(device_id, values).await?;
            }
        }
        Ok(())
    }

    async fn handle_device_connected(&self, device_id: usize, rig_model: String) -> Result<()> {
        self.rigs_state.write().insert(device_id, (rig_model, true));
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
        self.rigs_state
            .write()
            .entry(device_id)
            .and_modify(|(_, is_connected)| {
                *is_connected = false;
            });
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
}
