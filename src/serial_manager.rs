use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

use crate::commands::{BinaryParamArg, Command};
use crate::messaging::{DeviceMessage, MessageBroker, MessageHandler};
use crate::rig::RigSettings;
use crate::rig_file::RigFile;

struct SerialDevice {
    id: String,
    port: SerialStream,
    settings: Arc<RigSettings>,
    rig_file: Arc<RigFile>,
    command_tx: mpsc::Sender<DeviceCommand>,
}

#[derive(Debug)]
enum DeviceCommand {
    Execute {
        command_name: String,
        params: HashMap<String, BinaryParamArg>,
        response_tx: mpsc::Sender<Result<Vec<u8>>>,
    },
    Shutdown,
}

impl SerialDevice {
    async fn new(
        id: String,
        settings: Arc<RigSettings>,
        rig_file: Arc<RigFile>,
    ) -> Result<(Self, mpsc::Receiver<DeviceCommand>)> {
        let data_bits = match settings.data_bits {
            8 => tokio_serial::DataBits::Eight,
            7 => tokio_serial::DataBits::Seven,
            6 => tokio_serial::DataBits::Six,
            5 => tokio_serial::DataBits::Five,
            _ => return Err(anyhow::anyhow!("Invalid data bits")),
        };
        let stop_bits = match settings.stop_bits {
            1 => tokio_serial::StopBits::One,
            2 => tokio_serial::StopBits::Two,
            _ => return Err(anyhow::anyhow!("Invalid stop bits")),
        };
        let parity = if settings.parity {
            tokio_serial::Parity::Even
        } else {
            tokio_serial::Parity::None
        };
        let port = tokio_serial::new(&settings.port, settings.baud_rate)
            .data_bits(data_bits)
            .stop_bits(stop_bits)
            .parity(parity)
            .flow_control(tokio_serial::FlowControl::None)
            .open_native_async()
            .with_context(|| format!("Failed to open serial port {}", settings.port))?;

        let (command_tx, command_rx) = mpsc::channel(32);

        Ok((
            Self {
                id,
                port,
                settings,
                rig_file,
                command_tx,
            },
            command_rx,
        ))
    }

    async fn run(
        mut self,
        mut command_rx: mpsc::Receiver<DeviceCommand>,
        device_tx: broadcast::Sender<DeviceMessage>,
    ) -> Result<()> {
        device_tx
            .send(DeviceMessage::DeviceConnected {
                device_id: self.id.clone(),
            })
            .ok();

        while let Some(cmd) = command_rx.recv().await {
            match cmd {
                DeviceCommand::Execute {
                    command_name,
                    params,
                    response_tx,
                } => {
                    let result = self.execute_command(&command_name, params).await;

                    match result {
                        Ok(response) => {
                            response_tx.send(Ok(response.clone())).await.ok();
                            device_tx
                                .send(DeviceMessage::CommandResponse {
                                    device_id: self.id.clone(),
                                    command_name,
                                    response,
                                })
                                .ok();
                        }
                        Err(err) => {
                            let err_str = err.to_string();
                            response_tx.send(Err(err)).await.ok();
                            device_tx
                                .send(DeviceMessage::DeviceError {
                                    device_id: self.id.clone(),
                                    error: err_str,
                                })
                                .ok();
                        }
                    }
                }
                DeviceCommand::Shutdown => break,
            }
        }

        device_tx
            .send(DeviceMessage::DeviceDisconnected {
                device_id: self.id.clone(),
            })
            .ok();

        Ok(())
    }

    async fn execute_command(
        &mut self,
        command_name: &str,
        params: HashMap<String, BinaryParamArg>,
    ) -> Result<Vec<u8>> {
        let command = self
            .rig_file
            .commands
            .get(command_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown command: {}", command_name))?
            .clone();

        let reply_length = command.reply_length;

        let cmd = Command::try_from(command)?;
        let bytes = cmd.build_command(&params)?;

        self.port.write_all(&bytes).await?;

        let response = match reply_length {
            Some(length) => {
                let mut buf = vec![0u8; length as usize];
                self.port.read_exact(&mut buf).await?;
                buf
            }
            None => Vec::new(),
        };

        Ok(response)
    }
}

pub struct SerialManager {
    devices: Arc<Mutex<HashMap<String, mpsc::Sender<DeviceCommand>>>>,
    device_tx: broadcast::Sender<DeviceMessage>,
    broker: MessageBroker,
}

impl SerialManager {
    pub fn new(broker: MessageBroker) -> Self {
        Self {
            devices: Arc::new(Mutex::new(HashMap::new())),
            device_tx: broker.device_sender(),
            broker,
        }
    }

    async fn connect_device(&self, device_id: String, settings: Arc<RigSettings>) -> Result<()> {
        // TODO: Load from settings
        let rig_file = Arc::new(RigFile::new());
        let (device, command_rx) = SerialDevice::new(device_id.clone(), settings, rig_file).await?;

        self.devices
            .lock()
            .await
            .insert(device_id.clone(), device.command_tx.clone());

        let device_tx = self.device_tx.clone();
        tokio::spawn(async move {
            if let Err(err) = device.run(command_rx, device_tx.clone()).await {
                device_tx
                    .send(DeviceMessage::DeviceError {
                        device_id,
                        error: err.to_string(),
                    })
                    .ok();
            }
        });

        Ok(())
    }

    async fn disconnect_device(&self, device_id: &str) -> Result<()> {
        if let Some(command_tx) = self.devices.lock().await.remove(device_id) {
            command_tx.send(DeviceCommand::Shutdown).await?;
        }
        Ok(())
    }

    async fn execute_command(
        &self,
        device_id: &str,
        command_name: String,
        params: HashMap<String, BinaryParamArg>,
    ) -> Result<Vec<u8>> {
        let command_tx = self
            .devices
            .lock()
            .await
            .get(device_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Device not found: {}", device_id))?;

        let (response_tx, mut response_rx) = mpsc::channel(1);

        command_tx
            .send(DeviceCommand::Execute {
                command_name,
                params,
                response_tx,
            })
            .await?;

        response_rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Device disconnected"))??;

        // TODO: Return actual response
        Ok(vec![])
    }
}

#[async_trait::async_trait]
impl MessageHandler for SerialManager {
    async fn handle_messages(&mut self) -> Result<()> {
        let mut device_rx = self.broker.device_receiver();

        while let Ok(msg) = device_rx.recv().await {
            match msg {
                DeviceMessage::Connect {
                    device_id,
                    settings,
                } => {
                    if let Err(e) = self.connect_device(device_id.clone(), settings).await {
                        self.device_tx
                            .send(DeviceMessage::DeviceError {
                                device_id,
                                error: e.to_string(),
                            })
                            .ok();
                    }
                }
                DeviceMessage::Disconnect { device_id } => {
                    if let Err(e) = self.disconnect_device(&device_id).await {
                        self.device_tx
                            .send(DeviceMessage::DeviceError {
                                device_id,
                                error: e.to_string(),
                            })
                            .ok();
                    }
                }
                DeviceMessage::ExecuteCommand {
                    device_id,
                    command_name,
                    params,
                } => {
                    if let Err(e) = self.execute_command(&device_id, command_name, params).await {
                        self.device_tx
                            .send(DeviceMessage::DeviceError {
                                device_id,
                                error: e.to_string(),
                            })
                            .ok();
                    }
                }
                // Ignore other messages
                _ => {}
            }
        }

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        let device_ids: Vec<String> = self.devices.lock().await.keys().cloned().collect();
        for device_id in device_ids {
            self.disconnect_device(&device_id).await?;
        }
        Ok(())
    }
}
