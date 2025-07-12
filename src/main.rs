use anyhow::Result;
use eframe::egui;
use schema_parser::Config;
use tokio::sync::mpsc::{self, Receiver};

mod commands;
mod data_format;
mod gui;
mod omnirig_parser;
mod rig;
mod rig_file;
mod schema_parser;
mod serial;
mod translator;

use gui::{GuiMessage, SerialMessage};
use serial::manager::DeviceManager;

fn load_schema_file() -> Result<Config> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("holyrig")?;
    let config_path = xdg_dirs.place_config_file("schema.toml")?;
    Ok(schema_parser::parse_schema_file(config_path)?)
}

async fn serial_thread(
    gui_sender: mpsc::Sender<GuiMessage>,
    mut serial_receiver: Receiver<SerialMessage>,
) {
    let config = load_schema_file().unwrap();
    println!("Config: {config:#?}");

    let device_manager = DeviceManager::new();
    let manager_sender = device_manager.command_sender();

    let manager_sender_clone = manager_sender.clone();
    tokio::spawn(async move {
        while let Some(message) = serial_receiver.recv().await {
            match message {
                SerialMessage::ApplyRigConfig(rig_index, rig) => {
                    println!("Changed rig {rig_index}:\n{rig:#?}");
                }
            }
        }
    });

    if let Err(err) = device_manager.run(gui_sender).await {
        eprintln!("Device manager error: {err}");
    }
}

#[tokio::main]
async fn main() -> eframe::Result {
    let (gui_sender, gui_receiver) = mpsc::channel::<GuiMessage>(10);
    let (serial_sender, serial_receiver) = mpsc::channel::<SerialMessage>(10);

    tokio::spawn(async move { serial_thread(gui_sender, serial_receiver).await });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([350.0, 430.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "Holyrig",
        options,
        Box::new(|_| Ok(Box::new(gui::App::new(gui_receiver, serial_sender)))),
    )
}
