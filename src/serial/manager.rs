use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Duration, sleep};
use xdg::BaseDirectories;

use crate::commands::Value;
use crate::gui::GuiMessage;
use crate::rig::{RigSettings, Settings};
use crate::serial::device::{DeviceCommand, DeviceMessage, SerialDevice};
use crate::wrapper::{ExternalApi, RigWrapper};

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
    StatusUpdate {
        device_id: usize,
        values: HashMap<String, Value>,
    },
}

pub struct DeviceManager<W: RigWrapper + Clone + Send + Sync> {
    rigs: Arc<HashMap<String, W>>,
    devices: HashMap<usize, Device<W>>,
    settings: Settings,
    base_dirs: BaseDirectories,

    // manager -> ...
    manager_message_tx: broadcast::Sender<ManagerMessage>,

    // ... -> manager
    manager_command_tx: mpsc::Sender<ManagerCommand>,
    manager_command_rx: mpsc::Receiver<ManagerCommand>,

    // devices -> manager
    device_tx: mpsc::Sender<DeviceMessage>,
    device_rx: mpsc::Receiver<DeviceMessage>,
}

#[derive(Clone)]
struct Device<W: RigWrapper + Clone> {
    // Manager to devices channel
    command_tx: mpsc::Sender<DeviceCommand>,
    rig_wrapper: W,
    settings: RigSettings,
}

struct DeviceExternalApi {
    command_tx: mpsc::Sender<DeviceCommand>,
    status_values: Arc<Mutex<HashMap<String, Value>>>,
}

impl DeviceExternalApi {
    fn new(command_tx: mpsc::Sender<DeviceCommand>) -> Self {
        Self {
            command_tx,
            status_values: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_status_values(&self) -> HashMap<String, Value> {
        self.status_values.lock().unwrap().clone()
    }

    fn clear_status_values(&self) {
        self.status_values.lock().unwrap().clear();
    }
}

impl ExternalApi for DeviceExternalApi {
    async fn write(&self, data: &[u8]) -> Result<()> {
        self.command_tx
            .send(DeviceCommand::Write {
                data: data.to_vec(),
            })
            .await
            .context("Failed to send write command to device")
    }

    async fn read(&self, length: usize) -> Result<Vec<u8>> {
        let (read_tx, mut read_rx) = mpsc::channel(1);

        self.command_tx
            .send(DeviceCommand::ReadExact {
                length,
                response_tx: read_tx,
            })
            .await
            .context("Failed to send read command to device")?;

        read_rx
            .recv()
            .await
            .ok_or_else(|| anyhow!("Device disconnected"))?
    }

    fn set_var(&self, var: &str, value: Value) -> Result<()> {
        self.status_values
            .lock()
            .unwrap()
            .insert(var.to_string(), value);
        Ok(())
    }
}

impl<W: RigWrapper + Clone + Send + Sync + 'static> DeviceManager<W> {
    pub fn new(rigs: Arc<HashMap<String, W>>, base_dirs: BaseDirectories) -> Self {
        let (manager_command_tx, manager_command_rx) = mpsc::channel(10);
        let (device_tx, device_rx) = mpsc::channel(10);

        let (manager_message_tx, _) = broadcast::channel(10);

        Self {
            rigs,
            devices: HashMap::new(),
            settings: Default::default(),
            base_dirs,
            manager_message_tx,
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
            DeviceMessage::Connected { device_id } => {
                let _ = self
                    .manager_message_tx
                    .send(ManagerMessage::DeviceConnected { device_id });

                let init_result = self.initialize_device(device_id).await;
                if init_result.is_ok() {
                    self.start_status_polling(device_id).await
                } else {
                    init_result
                }
            }
            DeviceMessage::Disconnected { device_id } => {
                let _ = self
                    .manager_message_tx
                    .send(ManagerMessage::DeviceDisconnected { device_id });
                Ok(())
            }
            DeviceMessage::Error { device_id, error } => {
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
                    self.devices.remove(&settings.id);
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

    async fn execute_status_commands(device: &Device<W>) -> Result<HashMap<String, Value>> {
        let external_api = DeviceExternalApi::new(device.command_tx.clone());
        external_api.clear_status_values();
        device.rig_wrapper.execute_status(&external_api).await?;
        Ok(external_api.get_status_values())
    }

    async fn start_status_polling(&self, device_id: usize) -> Result<()> {
        let device = self
            .devices
            .get(&device_id)
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;

        let poll_interval = device.settings.poll_interval;
        let manager_tx = self.manager_message_tx.clone();
        let device_clone = device.clone();

        tokio::spawn(async move {
            let mut previous_values = HashMap::new();
            loop {
                sleep(Duration::from_millis(poll_interval as u64)).await;

                if let Ok(values) = Self::execute_status_commands(&device_clone).await {
                    let changed_values: HashMap<String, Value> = values
                        .iter()
                        .filter(|(name, value)| {
                            previous_values
                                .get(*name)
                                .map(|prev_value| prev_value != *value)
                                .unwrap_or(true)
                        })
                        .map(|(name, value)| (name.clone(), value.clone()))
                        .collect();
                    if !changed_values.is_empty() {
                        let _ = manager_tx.send(ManagerMessage::StatusUpdate {
                            device_id,
                            values: changed_values,
                        });
                    }
                    previous_values = values;
                }
            }
        });

        Ok(())
    }

    pub async fn add_device(&mut self, device_id: usize, settings: RigSettings) -> Result<()> {
        let rig_wrapper = self
            .rigs
            .get(&settings.rig_type)
            .context("Unknown rig type")?
            .clone();
        let (serial_device, command_rx) =
            SerialDevice::new(device_id, settings.clone(), self.device_tx.clone()).await?;

        let id = settings.id;

        let device = Device {
            command_tx: serial_device.command_sender(),
            rig_wrapper,
            settings,
        };

        self.devices.insert(device_id, device);

        let device_tx = self.device_tx.clone();
        tokio::spawn(async move {
            let device_id = id;

            device_tx
                .send(DeviceMessage::Connected { device_id })
                .await
                .unwrap();

            if let Err(err) = serial_device.run(command_rx).await {
                device_tx
                    .send(DeviceMessage::Error {
                        device_id,
                        error: err.to_string(),
                    })
                    .await
                    .unwrap();
            }
            device_tx
                .send(DeviceMessage::Disconnected { device_id })
                .await
                .unwrap();
        });

        Ok(())
    }

    async fn _remove_device(&mut self, device_id: usize) {
        if let Some(device) = self.devices.remove(&device_id) {
            let _ = device.command_tx.send(DeviceCommand::Shutdown).await;
        }
    }

    async fn execute_command(
        &self,
        device_id: usize,
        command_name: &str,
        params: HashMap<String, String>,
    ) -> Result<HashMap<String, Value>> {
        let device = self
            .devices
            .get(&device_id)
            .ok_or_else(|| anyhow!("Device not found: {}", device_id))?;

        let external_api = DeviceExternalApi::new(device.command_tx.clone());
        device
            .rig_wrapper
            .execute_command(command_name, params, &external_api)
            .await
    }

    pub async fn initialize_device(&self, device_id: usize) -> Result<()> {
        let device = self
            .devices
            .get(&device_id)
            .ok_or_else(|| anyhow!("Device not found: {device_id}"))?;

        let external_api = DeviceExternalApi::new(device.command_tx.clone());
        device.rig_wrapper.execute_init(&external_api).await
    }
}
