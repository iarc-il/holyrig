use crate::Value;
use anyhow::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;

use crate::serial::ManagerCommand;
use crate::serial::manager::ManagerMessage;

#[derive(Debug)]
enum RigctlCommand {
    SetFreq(f64),
    GetFreq,
    SetMode(String),
    GetMode,
    SetVfo(String),
    GetVfo,
    SetPtt(bool),
    GetPtt,
    SetSplit(bool),
    GetSplit,
    SetRit(i32),
    GetRit,
    SetXit(i32),
    GetXit,
    Quit,
}

fn parse_rigctl_command(line: &str) -> Result<RigctlCommand> {
    let mut chars = line.chars();
    let command_char = chars
        .next()
        .ok_or_else(|| anyhow::anyhow!("Empty command"))?;

    let args = chars.as_str().trim();

    match command_char {
        'F' => Ok(RigctlCommand::SetFreq(args.parse()?)),
        'f' => Ok(RigctlCommand::GetFreq),
        'M' => Ok(RigctlCommand::SetMode(args.to_string())),
        'm' => Ok(RigctlCommand::GetMode),
        'V' => Ok(RigctlCommand::SetVfo(args.to_string())),
        'v' => Ok(RigctlCommand::GetVfo),
        'T' => Ok(RigctlCommand::SetPtt(args.parse()?)),
        't' => Ok(RigctlCommand::GetPtt),
        'S' => Ok(RigctlCommand::SetSplit(args.parse()?)),
        's' => Ok(RigctlCommand::GetSplit),
        'J' => Ok(RigctlCommand::SetRit(args.parse()?)),
        'j' => Ok(RigctlCommand::GetRit),
        'Z' => Ok(RigctlCommand::SetXit(args.parse()?)),
        'z' => Ok(RigctlCommand::GetXit),
        'q' => Ok(RigctlCommand::Quit),
        _ => Err(anyhow::anyhow!("Unknown command: {}", command_char)),
    }
}

struct DeviceStatus {
    freq_a: i64,
    freq_b: i64,
    vfo: String,
    mode: String,
    transmit: bool,
    rit: bool,
    xit: bool,
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self {
            freq_a: 0,
            freq_b: 0,
            vfo: "A".to_string(),
            mode: "USB".to_string(),
            transmit: false,
            rit: false,
            xit: false,
        }
    }
}

pub async fn run_server(
    command_sender: Sender<ManagerCommand>,
    mut message_receiver: Receiver<ManagerMessage>,
) -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4532").await?;
    println!("Rigctld server listening on 127.0.0.1:4532");

    let device_status = Arc::new(RwLock::new(DeviceStatus::default()));

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                let (mut socket, addr) = accept_result?;
                let command_sender = command_sender.clone();

                let device_status = device_status.clone();
                tokio::spawn(async move {
                    let (reader, mut writer) = socket.split();
                    let mut reader = BufReader::new(reader);
                    let mut line = String::new();

                    loop {
                        line.clear();
                        if reader.read_line(&mut line).await.unwrap() == 0 {
                            break;
                        }

                        match parse_rigctl_command(&line) {
                            Ok(RigctlCommand::Quit) => break,
                            Ok(cmd) => {
                                let (command_name, params) = match cmd {
                                    RigctlCommand::SetFreq(freq) => {
                                        ("set_freq", {
                                            let mut p = HashMap::new();
                                            p.insert("freq".to_string(), freq.to_string());
                                            p.insert("target".to_string(), "Current".to_string());
                                            p
                                        })
                                    },
                                    RigctlCommand::GetFreq => {
                                        let freq = {
                                            let device_status = device_status.read();
                                            if device_status.vfo == "A" {
                                                device_status.freq_a
                                            } else {
                                                device_status.freq_b
                                            }
                                        };
                                        writer.write_all(format!("{}\n", freq).as_bytes()).await.unwrap();
                                        continue;
                                    },
                                    RigctlCommand::SetMode(mode) => {
                                        ("set_mode", {
                                            let mut p = HashMap::new();
                                            p.insert("mode".to_string(), mode);
                                            p
                                        })
                                    },
                                    RigctlCommand::GetMode => {
                                        let mode = { device_status.read().mode.clone() };
                                        writer.write_all(format!("{} 0\n", mode).as_bytes()).await.unwrap();
                                        continue;
                                    },
                                    RigctlCommand::SetVfo(vfo) => {
                                        ("set_vfo", {
                                            let mut p = HashMap::new();
                                            p.insert("rx".to_string(), vfo.clone());
                                            p.insert("tx".to_string(), vfo);
                                            p
                                        })
                                    },
                                    RigctlCommand::GetVfo => {
                                        let vfo = { device_status.read().vfo.clone() };
                                        writer.write_all(format!("VFO{}\n", vfo).as_bytes()).await.unwrap();
                                        continue;
                                    },
                                    RigctlCommand::SetPtt(ptt) => {
                                        ("transmit", HashMap::from([("tx".to_string(), ptt.to_string())]))
                                    },
                                    RigctlCommand::GetPtt => {
                                        let transmit = { device_status.read().transmit };
                                        writer.write_all(format!("{}\n", transmit as i32).as_bytes()).await.unwrap();
                                        continue;
                                    },
                                    RigctlCommand::SetSplit(split) => {
                                        ("set_split", {
                                            let mut p = HashMap::new();
                                            p.insert("split".to_string(), split.to_string());
                                            p
                                        })
                                    },
                                    RigctlCommand::GetSplit => {
                                        // Split status not available
                                        writer.write_all(b"0\n").await.unwrap();
                                        continue;
                                    },
                                    RigctlCommand::SetRit(_offset) => {
                                        // RIT offset not supported
                                        writer.write_all(b"RPRT -1\n").await.unwrap();
                                        continue;
                                    },
                                    RigctlCommand::GetRit => {
                                        let rit = { device_status.read().rit };
                                        writer.write_all(format!("{}\n", rit as i32).as_bytes()).await.unwrap();
                                        continue;
                                    },
                                    RigctlCommand::SetXit(_offset) => {
                                        // XIT offset not supported
                                        writer.write_all(b"RPRT -1\n").await.unwrap();
                                        continue;
                                    },
                                    RigctlCommand::GetXit => {
                                        let xit = { device_status.read().xit };
                                        writer.write_all(format!("{}\n", xit as i32).as_bytes()).await.unwrap();
                                        continue;
                                    },
                                    RigctlCommand::Quit => unreachable!(),
                                };

                                if let Err(e) = command_sender
                                    .send(ManagerCommand::ExecuteCommand {
                                        device_id: 0, // TODO: Support multiple devices
                                        command_name: command_name.to_string(),
                                        params,
                                    })
                                    .await
                                {
                                    eprintln!("Failed to send command: {}", e);
                                    break;
                                }

                                writer.write_all(b"RPRT 0\n").await.unwrap();
                            },
                            Err(e) => {
                                eprintln!("Error parsing command from {}: {}", addr, e);
                                writer.write_all(b"RPRT 1\n").await.unwrap();
                            }
                        }
                    }
                });
            }
            Ok(msg) = message_receiver.recv() => {
                let device_status = device_status.clone();
                if let ManagerMessage::StatusUpdate { values, .. } = msg {
                    let mut device_status = device_status.write();
                    for (name, value) in values {
                        match (name.as_str(), value) {
                            ("freq_a", Value::Integer(f)) => device_status.freq_a = f,
                            ("freq_b", Value::Integer(f)) => device_status.freq_b = f,
                            ("vfo", Value::String(v)) => device_status.vfo = v,
                            ("mode", Value::String(m)) => device_status.mode = m,
                            ("transmit", Value::Boolean(t)) => device_status.transmit = t,
                            ("rit", Value::Boolean(r)) => device_status.rit = r,
                            ("xit", Value::Boolean(x)) => device_status.xit = x,
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
