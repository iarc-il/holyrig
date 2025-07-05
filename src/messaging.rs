use std::sync::Arc;
use tokio::sync::broadcast;
use anyhow::Result;
use std::collections::HashMap;

use crate::rig::RigSettings;
use crate::commands::BinaryParamArg;

#[derive(Debug, Clone)]
pub enum DeviceMessage {
    ExecuteCommand {
        device_id: String,
        command_name: String,
        params: HashMap<String, BinaryParamArg>,
    },
    CommandResponse {
        device_id: String,
        command_name: String,
        response: Vec<u8>,
    },
    Connect {
        device_id: String,
        settings: Arc<RigSettings>,
    },
    Disconnect {
        device_id: String,
    },
    // Status
    DeviceConnected {
        device_id: String,
    },
    DeviceDisconnected {
        device_id: String,
    },
    DeviceError {
        device_id: String,
        error: String,
    },
}

#[derive(Debug, Clone)]
pub enum GuiMessage {
    UpdateSettings { settings: Arc<RigSettings> },
    SettingsChanged { settings: Arc<RigSettings> },

    ShowError { message: String },
    UpdateStatus { status: String },

    OpenWindow { window_type: String },
    CloseWindow { window_type: String },
}

pub struct MessageBroker {
    device_tx: broadcast::Sender<DeviceMessage>,
    gui_tx: broadcast::Sender<GuiMessage>,
}

impl MessageBroker {
    pub fn new(capacity: usize) -> Self {
        let (device_tx, _) = broadcast::channel(capacity);
        let (gui_tx, _) = broadcast::channel(capacity);

        Self { device_tx, gui_tx }
    }

    pub fn device_sender(&self) -> broadcast::Sender<DeviceMessage> {
        self.device_tx.clone()
    }

    pub fn gui_sender(&self) -> broadcast::Sender<GuiMessage> {
        self.gui_tx.clone()
    }

    pub fn device_receiver(&self) -> broadcast::Receiver<DeviceMessage> {
        self.device_tx.subscribe()
    }

    pub fn gui_receiver(&self) -> broadcast::Receiver<GuiMessage> {
        self.gui_tx.subscribe()
    }
}

#[async_trait::async_trait]
pub trait MessageHandler {
    async fn handle_messages(&mut self) -> Result<()>;
    async fn shutdown(&mut self) -> Result<()>;
}
