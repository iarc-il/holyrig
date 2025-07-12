use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

use crate::commands::{Command, Value};
use crate::gui::GuiMessage;
use crate::rig::RigSettings;
use crate::rig_file::RigFile;
use crate::serial::device::{DeviceCommand, DeviceMessage, SerialDevice};

#[derive(Debug, Clone)]
pub enum CommandResponse {
    Success,
}

#[derive(Debug)]
pub enum ManagerCommand {
    CreateOrUpdateDevice {
        device_id: String,
        settings: RigSettings,
    },
}

#[derive(Debug, Clone)]
pub enum ManagerMessage {
    CommandResponse {
        device_id: String,
        command_name: String,
        response: CommandResponse,
    },
}

pub struct DeviceManager {
    devices: HashMap<String, DeviceState>,

    // Manager data output channel
    manager_message_rx: broadcast::Receiver<ManagerMessage>,
    manager_message_tx: broadcast::Sender<ManagerMessage>,

    // Manager command input channel
    manager_command_tx: mpsc::Sender<ManagerCommand>,
    manager_command_rx: mpsc::Receiver<ManagerCommand>,

    // Devices to manager channel
    device_tx: mpsc::Sender<DeviceMessage>,
    device_rx: mpsc::Receiver<DeviceMessage>,
}

struct DeviceState {
    // Manager to devices channel
    command_tx: mpsc::Sender<DeviceCommand>,
    rig_file: Arc<RigFile>,
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceManager {
    pub fn new() -> Self {
        let (manager_command_tx, manager_command_rx) = mpsc::channel(10);
        let (device_tx, device_rx) = mpsc::channel(10);

        let (manager_message_tx, manager_message_rx) = broadcast::channel(10);

        Self {
            devices: HashMap::new(),
            manager_message_tx,
            manager_message_rx,
            manager_command_tx,
            manager_command_rx,
            device_tx,
            device_rx,
        }
    }

    pub fn message_sender(&self) -> broadcast::Sender<ManagerMessage> {
        self.manager_message_tx.clone()
    }

    pub fn command_sender(&self) -> mpsc::Sender<ManagerCommand> {
        self.manager_command_tx.clone()
    }

    pub async fn run(mut self, gui_sender: mpsc::Sender<GuiMessage>) -> Result<()> {
        loop {
            tokio::select! {
                Some(device_msg) = self.device_rx.recv() => {
                    match device_msg {
                        DeviceMessage::DeviceConnected { device_id } => {
                            println!("Device connected: {device_id}");
                            if let Err(err) = self.initialize_device(&device_id).await {
                                eprintln!("Failed to initialize device {device_id}: {err}");
                                let _ = self.remove_device(&device_id).await;
                            }
                        }
                        DeviceMessage::DeviceDisconnected { device_id } => {
                            println!("Device disconnected: {device_id}");
                            let _ = self.remove_device(&device_id).await;
                        }
                        DeviceMessage::DeviceError { device_id, error } => {
                            eprintln!("Device error for {device_id}: {error}");
                            let _ = self.remove_device(&device_id).await;
                        }
                    }
                }
                Some(cmd) = self.manager_command_rx.recv() => {
                    match cmd {
                        ManagerCommand::CreateOrUpdateDevice { device_id, settings } => {
                            println!("Got create or update device: {device_id}, {settings:?}");
                            if let Some(device) = self.devices.get(&device_id) {
                                todo!()
                            } else {
                                self.add_device(device_id, settings, todo!());
                            }
                        },
                    }
                }
            }
        }
    }

    pub async fn add_device(
        &mut self,
        device_id: String,
        settings: RigSettings,
        rig_file: RigFile,
    ) -> Result<()> {
        let rig_file = Arc::new(rig_file);

        let (device, command_rx) = SerialDevice::new(device_id.clone(), settings).await?;

        let device_state = DeviceState {
            command_tx: device.command_sender(),
            rig_file: rig_file.clone(),
        };

        self.devices.insert(device_id.clone(), device_state);

        let device_tx = self.device_tx.clone();
        tokio::spawn(async move {
            let device_id = device.id().to_string();

            if let Err(err) = device.run(command_rx).await {
                device_tx
                    .send(DeviceMessage::DeviceError {
                        device_id: device_id.clone(),
                        error: err.to_string(),
                    })
                    .await
                    .unwrap();
            }
            device_tx
                .send(DeviceMessage::DeviceDisconnected { device_id })
                .await
                .unwrap();
        });

        self.device_tx
            .send(DeviceMessage::DeviceConnected { device_id })
            .await?;

        Ok(())
    }

    pub async fn remove_device(&mut self, device_id: &str) -> Result<()> {
        if let Some(state) = self.devices.remove(device_id) {
            state.command_tx.send(DeviceCommand::Shutdown).await?;
        }
        Ok(())
    }

    pub async fn execute_command(
        &self,
        device_id: &str,
        command_name: &str,
        params: HashMap<String, Value>,
    ) -> Result<Vec<u8>> {
        let state = self
            .devices
            .get(device_id)
            .ok_or_else(|| anyhow::anyhow!("Device not found: {}", device_id))?;

        let command = state
            .rig_file
            .commands
            .get(command_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown command: {}", command_name))?
            .clone();

        let cmd = Command::try_from(command)?;
        let bytes = cmd.build_command(&params)?;

        let (response_tx, mut response_rx) = mpsc::channel(1);

        let expected_length = cmd.response_length();
        state
            .command_tx
            .send(DeviceCommand::Write {
                data: bytes,
                expected_length,
                response_tx,
            })
            .await
            .context("Failed to send command to device")?;

        let response = response_rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Device disconnected"))??;

        self.manager_message_tx
            .send(ManagerMessage::CommandResponse {
                device_id: device_id.to_string(),
                command_name: command_name.to_string(),
                response: CommandResponse::Success,
            })?;

        Ok(response)
    }

    pub async fn initialize_device(&self, device_id: &str) -> Result<()> {
        let state = self
            .devices
            .get(device_id)
            .ok_or_else(|| anyhow::anyhow!("Device not found: {}", device_id))?;

        for command in &state.rig_file.init {
            let command = Command::try_from(command.clone())?;
            let bytes = command.build_command(&HashMap::new())?;

            let (response_tx, mut response_rx) = mpsc::channel(1);

            let expected_length = command.response_length();
            state
                .command_tx
                .send(DeviceCommand::Write {
                    data: bytes,
                    expected_length,
                    response_tx,
                })
                .await
                .context("Failed to send init command to device")?;

            response_rx
                .recv()
                .await
                .ok_or_else(|| anyhow::anyhow!("Device disconnected"))??;
        }

        Ok(())
    }
}
