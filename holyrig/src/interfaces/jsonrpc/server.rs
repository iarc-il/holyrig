use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, mpsc};

use super::{Notification, RigRpcHandler};
use crate::interfaces::jsonrpc::{Request, Response, RpcError};
use crate::resources::Resources;
use crate::serial::manager::{ManagerCommand, ManagerMessage};

type Subscriptions = HashMap<(usize, SocketAddr), Vec<String>>;

pub struct JsonRpcServer {
    bind_address: String,
    port: u16,
    handlers: Arc<HashMap<String, RigRpcHandler>>,
    rigs_state: Arc<RwLock<HashMap<usize, (String, bool)>>>,
    subscribed_status: Arc<RwLock<Subscriptions>>,
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
            subscribed_status: Arc::new(RwLock::new(HashMap::new())),
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
                                let response = match self.handle_packet(&buf[..len], src_addr).await {
                                    Ok(response) => response,
                                    Err(err) => {
                                        if let Some(rpc_error) = err.downcast_ref::<RpcError>() {
                                            eprintln!("Error handling UDP datagram: {err}");
                                            Response::build_error(rpc_error.clone())
                                        } else {
                                            continue;
                                        }
                                    },
                                };
                                let error_data = serde_json::to_vec(&response).unwrap();
                                socket.send_to(&error_data, src_addr).await.unwrap();
                            },
                            Err(err) => {
                                eprintln!("Failed to receive data: {err}");
                            }
                        }
                    }
                    message = self.manager_rx.recv() => {
                        let message = message.unwrap();
                        if let Err(err) = self.handle_manager_message(message, &socket).await {
                            eprintln!("Error handling manager message: {err}");
                        }
                    }
                }
            }
        });
        Ok(())
    }

    async fn handle_packet(&self, data: &[u8], src_addr: SocketAddr) -> Result<Response> {
        let request = serde_json::from_slice::<Request>(data)
            .map_err(|err| anyhow!(RpcError::parse_error(&err)))?;
        let response = match request.method.as_str() {
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
            "subscribe_status" => {
                let id = request
                    .get_rig_id()
                    .ok_or_else(|| anyhow!(RpcError::missing_rig_id()))?;
                let fields = request
                    .params
                    .as_ref()
                    .and_then(|params| params.as_object())
                    .and_then(|params| params.get("fields"))
                    .and_then(|fields| fields.as_array())
                    .and_then(|fields| {
                        fields
                            .iter()
                            .map(|field| field.as_str().map(|field| field.to_string()))
                            .collect::<Option<Vec<_>>>()
                    })
                    .ok_or_else(|| anyhow!(RpcError::invalid_params().with_id(&request.id)))?;

                self.subscribed_status
                    .write()
                    .insert((id, src_addr), fields);

                Response::build_success(request.id)
            }
            _ => {
                let id = request
                    .get_rig_id()
                    .ok_or_else(|| anyhow!(RpcError::missing_rig_id().with_id(&request.id)))?;
                let handler = {
                    let rigs = self.rigs_state.read();
                    let (rig_model, _) = rigs.get(&id).unwrap();
                    self.handlers
                        .get(rig_model)
                        .ok_or_else(|| anyhow!(RpcError::unknown_rig_id(id).with_id(&request.id)))?
                };
                handler.handle_request(&request, id).await?
            }
        };

        Ok(response)
    }

    async fn handle_manager_message(
        &self,
        message: ManagerMessage,
        socket: &UdpSocket,
    ) -> Result<()> {
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
                self.rigs_state.write().insert(device_id, (rig_model, true));
            }
            ManagerMessage::DeviceDisconnected { device_id } => {
                self.rigs_state
                    .write()
                    .entry(device_id)
                    .and_modify(|(_, is_connected)| {
                        *is_connected = false;
                    });
            }
            ManagerMessage::StatusUpdate { device_id, values } => {
                let values: HashMap<_, _> = values
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::from(v)))
                    .collect();

                let clients: Vec<_> = self
                    .subscribed_status
                    .read()
                    .clone()
                    .into_iter()
                    .filter(|((id, _), _)| *id == device_id)
                    .collect();

                for ((_, addr), fields) in clients {
                    let values: HashMap<_, _> = values
                        .iter()
                        .filter_map(|(k, v)| {
                            if fields.contains(k) {
                                Some((k.clone(), v.clone()))
                            } else {
                                None
                            }
                        })
                        .collect();

                    let notification = Notification {
                        jsonrpc: super::VERSION.into(),
                        method: "status_update".to_string(),
                        params: json!({
                            "rig_id": device_id,
                            "updates": values,
                        }),
                    };
                    let packet = serde_json::to_vec(&notification).unwrap();
                    if let Err(err) = socket.send_to(&packet, addr).await {
                        eprintln!("Failed to send notification to {addr}: {err}");
                        self.subscribed_status.write().remove(&(device_id, addr));
                    }
                }
            }
        }
        Ok(())
    }
}
