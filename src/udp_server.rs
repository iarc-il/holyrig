use anyhow::{Context, Result};
use std::collections::HashMap;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;

use crate::commands::Value;
use crate::schema::Schema;
use crate::serial::ManagerCommand;

// Parse a command string in format: "DEVICE_ID COMMAND_NAME PARAM1=VALUE1 PARAM2=VALUE2"
fn parse_command(cmd: &str, schema: &Schema) -> Result<(String, String, HashMap<String, Value>)> {
    let mut parts = cmd.split_whitespace();

    let device_id = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("Missing device ID"))?
        .to_string();

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
        params.insert(key.to_string(), value_type.build_value(value));
    }

    Ok((device_id, command_name, params))
}

pub async fn run_server(command_sender: Sender<ManagerCommand>, schema: &Schema) -> Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:8888").await?;
    println!("UDP debug interface listening on 127.0.0.1:8888");

    let mut buf = [0; 1024];

    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        let cmd = String::from_utf8_lossy(&buf[..len]);

        match parse_command(&cmd, schema) {
            Ok((device_id, command_name, params)) => {
                println!("Received command from {addr}: {device_id} {command_name} {params:?}");

                command_sender
                    .send(ManagerCommand::ExecuteCommand {
                        device_id,
                        command_name,
                        params,
                    })
                    .await?;

                // TODO wait for response
                // match device_manager
                //     .execute_command(&device_id, &command_name, params)
                //     .await
                // {
                //     Ok(response) => {
                //         let response_str = format!("OK: {:?}", response);
                //         socket.send_to(response_str.as_bytes(), addr).await?;
                //     }
                //     Err(e) => {
                //         let error_str = format!("ERROR: {}", e);
                //         socket.send_to(error_str.as_bytes(), addr).await?;
                //     }
                // }
            }
            Err(err) => {
                let error_str = format!("ERROR: Invalid command format - {err}");
                socket.send_to(error_str.as_bytes(), addr).await?;
            }
        }
    }
}
