use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

use crate::rig::{DataBits, RigSettings, StopBits};

#[derive(Debug)]
pub enum DeviceCommand {
    Write {
        data: Vec<u8>,
        expected_length: Option<usize>,
        response_tx: mpsc::Sender<Result<Vec<u8>>>,
    },
    Shutdown,
}

#[derive(Debug)]
pub enum DeviceMessage {
    DeviceError { device_id: usize, error: String },
    DeviceDisconnected { device_id: usize },
    DeviceConnected { device_id: usize },
}

pub struct SerialDevice {
    id: usize,
    port: SerialStream,
    settings: RigSettings,
    command_tx: mpsc::Sender<DeviceCommand>,
}

impl SerialDevice {
    pub async fn new(
        id: usize,
        settings: RigSettings,
    ) -> Result<(Self, mpsc::Receiver<DeviceCommand>)> {
        let data_bits = match settings.data_bits {
            DataBits::Bits8 => tokio_serial::DataBits::Eight,
            DataBits::Bits7 => tokio_serial::DataBits::Seven,
            DataBits::Bits6 => tokio_serial::DataBits::Six,
            DataBits::Bits5 => tokio_serial::DataBits::Five,
        };
        let stop_bits = match settings.stop_bits {
            StopBits::Bits1 => tokio_serial::StopBits::One,
            StopBits::Bits2 => tokio_serial::StopBits::Two,
        };
        let parity = if settings.parity {
            tokio_serial::Parity::Even
        } else {
            tokio_serial::Parity::None
        };
        let port = tokio_serial::new(&settings.port, settings.baud_rate.into())
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
                command_tx,
            },
            command_rx,
        ))
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn command_sender(&self) -> mpsc::Sender<DeviceCommand> {
        self.command_tx.clone()
    }

    pub async fn run(mut self, mut command_rx: mpsc::Receiver<DeviceCommand>) -> Result<()> {
        while let Some(cmd) = command_rx.recv().await {
            match cmd {
                DeviceCommand::Write {
                    data,
                    expected_length,
                    response_tx,
                } => {
                    let result = self.write_and_read(&data, expected_length).await;
                    response_tx.send(result).await.ok();
                }
                DeviceCommand::Shutdown => break,
            }
        }
        Ok(())
    }

    async fn write_and_read(
        &mut self,
        data: &[u8],
        expected_length: Option<usize>,
    ) -> Result<Vec<u8>> {
        self.port.write_all(data).await?;

        match expected_length {
            Some(length) => {
                let mut buf = vec![0u8; length];
                self.port.read_exact(&mut buf).await?;
                Ok(buf)
            }
            None => Ok(Vec::new()),
        }
    }
}
