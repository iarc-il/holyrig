use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use xdg::BaseDirectories;

use crate::commands::Value;
use crate::gui::GuiMessage;
use crate::rig::{RigSettings, Settings};
use crate::rig_api::RigApi;
use crate::serial::device::{DeviceCommand, DeviceMessage, SerialDevice};

const RIGS_FILE: &str = "rigs.toml";

#[derive(Debug, Clone)]
pub enum CommandResponse {
    Success(HashMap<String, Value>),
    Error(String),
}

#[derive(Debug)]
pub enum ManagerCommand {
    CreateOrUpdateDevice {
        settings: RigSettings,
    },
    ExecuteCommand {
        device_id: usize,
        command_name: String,
        params: HashMap<String, String>,
    },
    RemoveDevice {
        device_id: usize,
    },
}

#[derive(Debug, Clone)]
pub enum ManagerMessage {
    CommandResponse {
        device_id: usize,
        command_name: String,
        response: CommandResponse,
    },
    DeviceConnected {
        device_id: usize,
    },
    DeviceDisconnected {
        device_id: usize,
    },
}

pub struct DeviceManager {
    rigs: Arc<HashMap<String, RigApi>>,
    devices: HashMap<usize, Device>,
    settings: Settings,
    base_dirs: BaseDirectories,

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

struct Device {
    // Manager to devices channel
    command_tx: mpsc::Sender<DeviceCommand>,
    rig_api: RigApi,
}

impl DeviceManager {
    pub fn new(rigs: Arc<HashMap<String, RigApi>>, base_dirs: BaseDirectories) -> Self {
        let (manager_command_tx, manager_command_rx) = mpsc::channel(10);
        let (device_tx, device_rx) = mpsc::channel(10);

        let (manager_message_tx, manager_message_rx) = broadcast::channel(10);

        Self {
            rigs,
            devices: HashMap::new(),
            settings: Default::default(),
            base_dirs,
            manager_message_tx,
            manager_message_rx,
            manager_command_tx,
            manager_command_rx,
            device_tx,
            device_rx,
        }
    }

    pub fn receiver(&self) -> broadcast::Receiver<ManagerMessage> {
        self.manager_message_tx.subscribe()
    }

    pub fn sender(&self) -> mpsc::Sender<ManagerCommand> {
        self.manager_command_tx.clone()
    }

    pub async fn load_rigs(&mut self, gui_sender: &mpsc::Sender<GuiMessage>) -> Result<()> {
        let settings_path = self.base_dirs.get_data_file(RIGS_FILE);
        let settings = if !settings_path.exists() {
            Settings::default()
        } else {
            let content = std::fs::read_to_string(&settings_path)?;
            toml::from_str(&content)?
        };

        for (rig_id, settings) in settings.rigs.iter().enumerate() {
            if let Err(err) = self.add_device(rig_id, settings.clone()).await {
                eprintln!("Failed to load rig {rig_id}: {err}");
            }
        }

        gui_sender
            .send(GuiMessage::InitialState(settings.rigs.clone()))
            .await?;

        self.settings = settings;

        Ok(())
    }

    async fn handle_device_message(&mut self, device_message: DeviceMessage) {
        let result = match device_message {
            DeviceMessage::DeviceConnected { device_id } => {
                let _ = self
                    .manager_message_tx
                    .send(ManagerMessage::DeviceConnected { device_id });
                self.initialize_device(device_id).await
            }
            DeviceMessage::DeviceDisconnected { device_id } => {
                let _ = self
                    .manager_message_tx
                    .send(ManagerMessage::DeviceDisconnected { device_id });
                Ok(())
            }
            DeviceMessage::DeviceError { device_id, error } => {
                Err(anyhow!("Device (id: {device_id}) failed: {error}"))
            }
        };
        if let Err(err) = result {
            eprintln!("{err}");
        }
    }

    async fn handle_manager_command(&mut self, manager_command: ManagerCommand) -> Result<()> {
        match manager_command {
            ManagerCommand::CreateOrUpdateDevice { settings } => {
                if let Some(_device) = self.devices.get(&settings.id) {
                    todo!()
                } else {
                    let changed_settings = self
                        .settings
                        .rigs
                        .iter_mut()
                        .find(|rig| rig.id == settings.id);
                    if let Some(changed_settings) = changed_settings {
                        *changed_settings = settings.clone();
                    } else {
                        self.settings.rigs.push(settings.clone());
                    };
                    let path = self.base_dirs.place_data_file(RIGS_FILE)?;
                    let content = toml::to_string(&self.settings)?;
                    std::fs::write(path, content)?;

                    self.add_device(settings.id, settings).await?;
                }
            }
            ManagerCommand::ExecuteCommand {
                device_id,
                command_name,
                params,
            } => {
                let result = self.execute_command(device_id, &command_name, params).await;

                let response = match result {
                    Ok(response) => CommandResponse::Success(response),
                    Err(err) => {
                        eprintln!("Command {command_name} of device {device_id} failed: {err}");
                        CommandResponse::Error(err.to_string())
                    }
                };
                self.manager_message_tx
                    .send(ManagerMessage::CommandResponse {
                        device_id,
                        command_name,
                        response,
                    })?;
            }
            ManagerCommand::RemoveDevice { device_id } => {
                if let Some(device) = self.devices.remove(&device_id) {
                    let _ = device.command_tx.send(DeviceCommand::Shutdown).await;
                }

                if let Some(pos) = self
                    .settings
                    .rigs
                    .iter()
                    .position(|rig| rig.id == device_id)
                {
                    self.settings.rigs.remove(pos);
                    let path = self.base_dirs.place_data_file(RIGS_FILE)?;
                    let content = toml::to_string(&self.settings)?;
                    std::fs::write(path, content)?;
                }
            }
        }
        Ok(())
    }

    pub async fn run(&mut self, gui_sender: mpsc::Sender<GuiMessage>) -> Result<()> {
        self.load_rigs(&gui_sender).await?;

        loop {
            tokio::select! {
                Some(device_message) = self.device_rx.recv() => {
                    self.handle_device_message(device_message).await;
                },
                Some(manager_command) = self.manager_command_rx.recv() => {
                    self.handle_manager_command(manager_command).await?
                },
            }
        }
    }

    pub async fn add_device(&mut self, device_id: usize, settings: RigSettings) -> Result<()> {
        let rig_api = self
            .rigs
            .get(&settings.rig_type)
            .context("Unknown rig type")?
            .clone();
        let (device, command_rx) =
            SerialDevice::new(device_id, settings, self.device_tx.clone()).await?;

        let device_state = Device {
            command_tx: device.command_sender(),
            rig_api,
        };

        self.devices.insert(device_id, device_state);

        let device_tx = self.device_tx.clone();
        tokio::spawn(async move {
            let device_id = device.id();

            if let Err(err) = device.run(command_rx).await {
                device_tx
                    .send(DeviceMessage::DeviceError {
                        device_id,
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

    async fn _remove_device(&mut self, device_id: usize) {
        if let Some(state) = self.devices.remove(&device_id) {
            let _ = state.command_tx.send(DeviceCommand::Shutdown).await;
        }
    }

    async fn execute_command(
        &self,
        device_id: usize,
        command_name: &str,
        params: HashMap<String, String>,
    ) -> Result<HashMap<String, Value>> {
        let state = self
            .devices
            .get(&device_id)
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;

        let params = state.rig_api.parse_param_values(command_name, params)?;
        let bytes = state.rig_api.build_command(command_name, &params)?;
        let expected_length = state.rig_api.get_command_response_length(command_name)?;

        let (response_tx, mut response_rx) = mpsc::channel(1);

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
            .ok_or_else(|| anyhow!("Device disconnected"))??;

        state
            .rig_api
            .parse_command_response(command_name, &response)
            .map_err(|err| anyhow!(err))
    }

    pub async fn initialize_device(&self, device_id: usize) -> Result<()> {
        let state = self
            .devices
            .get(&device_id)
            .ok_or_else(|| anyhow!("Device not found: {device_id}"))?;

        for (index, data) in state.rig_api.build_init_commands()?.into_iter().enumerate() {
            let expected_length = state.rig_api.get_init_response_length(index)?;

            let (response_tx, mut response_rx) = mpsc::channel(1);
            state
                .command_tx
                .send(DeviceCommand::Write {
                    data,
                    expected_length,
                    response_tx,
                })
                .await
                .context("Failed to send init command to device")?;

            response_rx
                .recv()
                .await
                .ok_or_else(|| anyhow!("Device disconnected"))??;
        }

        Ok(())
    }
}
