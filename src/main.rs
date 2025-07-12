use anyhow::Result;
use eframe::egui;
use schema_parser::Config;
use tokio::sync::mpsc;

mod commands;
mod data_format;
mod gui;
mod omnirig_parser;
mod rig;
mod rig_file;
mod schema_parser;
mod serial;
mod translator;

use gui::GuiMessage;
use serial::manager::DeviceManager;

fn load_schema_file() -> Result<Config> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("holyrig")?;
    let config_path = xdg_dirs.place_config_file("schema.toml")?;
    Ok(schema_parser::parse_schema_file(config_path)?)
}

async fn serial_thread(gui_sender: mpsc::Sender<GuiMessage>, device_manager: DeviceManager) {
    let config = load_schema_file().unwrap();
    println!("Config: {config:#?}");

    if let Err(err) = device_manager.run(gui_sender).await {
        eprintln!("Device manager error: {err}");
    }
}

#[tokio::main]
async fn main() -> eframe::Result {
    let (gui_sender, gui_receiver) = mpsc::channel::<GuiMessage>(10);
    let device_manager = DeviceManager::new();

    let manager_command_sender = device_manager.command_sender();

    tokio::spawn(async move { serial_thread(gui_sender, device_manager).await });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([350.0, 430.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "Holyrig",
        options,
        Box::new(|_| {
            Ok(Box::new(gui::App::new(
                gui_receiver,
                manager_command_sender,
            )))
        }),
    )
}
