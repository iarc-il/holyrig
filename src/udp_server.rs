use anyhow::{Context, Result};
use std::collections::HashMap;
use tokio::net::UdpSocket;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;

use crate::commands::Value;
use crate::schema::Schema;
use crate::serial::ManagerCommand;
use crate::serial::manager::{CommandResponse, ManagerMessage};

// Parse a command string in format: "DEVICE_ID COMMAND_NAME PARAM1=VALUE1 PARAM2=VALUE2"
fn parse_command(cmd: &str, schema: &Schema) -> Result<(usize, String, HashMap<String, Value>)> {
    let mut parts = cmd.split_whitespace();

    let device_id = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("Missing device ID"))?
        .parse()?;

    let command_name = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("Missing command name"))?
        .to_string();

    let mut params = HashMap::new();
    for param in parts {
        let mut kv = param.split('=');
        let key = kv
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid parameter format"))?;
        let value = kv
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid parameter format"))?;

        let (_, value_type) = schema
            .commands
            .get(&command_name)
            .context("Unknown command")?
            .params
            .iter()
            .find(|(name, _)| name == key)
            .context("Unknown param")?;
        params.insert(key.to_string(), value_type.build_value(value)?);
    }

    Ok((device_id, command_name, params))
}

pub async fn run_server(
    command_sender: Sender<ManagerCommand>,
    mut message_receiver: Receiver<ManagerMessage>,
    schema: &Schema,
) -> Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:8888").await?;
    println!("UDP debug interface listening on 127.0.0.1:8888");

    let mut buf = [0; 1024];

    let mut device_id_to_addr = HashMap::new();

    loop {
        let (len, addr) = tokio::select! {
            result = socket.recv_from(&mut buf) => {
                result?
            },
            response = message_receiver.recv() => {
                let (udp_response, device_id) = match response? {
                    ManagerMessage::CommandResponse { device_id, command_name, response } => {
                         let response = match response {
                             CommandResponse::Success => {
                                 format!("Executed command {command_name} on device {device_id}\n")
                             },
                             CommandResponse::Error(err) => {
                                 format!("Failed executing command {command_name} on device {device_id}: {err}\n")

                             },
                         };
                         (response, device_id)
                    },
                    ManagerMessage::DeviceConnected { device_id } => {
                        (format!("Device {device_id} connected"), device_id)
                    },
                    ManagerMessage::DeviceDisconnected { device_id } => {
                        (format!("Device {device_id} disconnected"), device_id)
                    },
                };
                if let Some(addr) = device_id_to_addr.get(&device_id) {
                    socket.send_to(udp_response.as_bytes(), addr).await?;
                }
                continue;
            }
        };

        let cmd = String::from_utf8_lossy(&buf[..len]);

        match parse_command(&cmd, schema) {
            Ok((device_id, command_name, params)) => {
                println!("Received command from {addr}: {device_id} {command_name} {params:?}");

                device_id_to_addr.insert(device_id, addr);

                command_sender
                    .send(ManagerCommand::ExecuteCommand {
                        device_id,
                        command_name,
                        params,
                    })
                    .await?;
            }
            Err(err) => {
                let error_str = format!("ERROR: Invalid command format - {err}\n");
                socket.send_to(error_str.as_bytes(), addr).await?;
            }
        }
    }
}
