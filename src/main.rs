use anyhow::Result;
use eframe::egui;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

mod commands;
mod data_format;
mod gui;
mod omnirig_parser;
mod rig;
mod rig_api;
mod rig_file;
mod schema;
mod serial;

use gui::GuiMessage;
use rig_api::RigApi;
use rig_file::RigFile;
use serial::manager::DeviceManager;

fn load_rig_files<P: AsRef<Path>>(dir_path: P) -> Result<Arc<HashMap<String, RigApi>>> {
    let mut rigs = HashMap::new();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }

        let file_name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?
            .to_string();

        let content = fs::read_to_string(&path)?;
        let rig_file: RigFile = toml::from_str(&content)?;

        match RigApi::try_from(rig_file) {
            Ok(rig_api) => {
                rigs.insert(file_name, rig_api);
            }
            Err(err) => {
                eprintln!("Failed to load rig file {}: {}", path.display(), err);
            }
        }
    }

    Ok(Arc::new(rigs))
}

fn load_schema_file() -> Result<schema::Schema> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("holyrig")?;
    let config_path = xdg_dirs.place_config_file("schema.toml")?;
    schema::Schema::load(config_path).map_err(|e| anyhow::anyhow!("Failed to load schema: {}", e))
}

async fn serial_thread(gui_sender: mpsc::Sender<GuiMessage>, device_manager: DeviceManager) {
    match load_schema_file() {
        Ok(config) => {
            println!("Loaded schema: {config:#?}");
            if let Err(err) = device_manager.run(gui_sender).await {
                eprintln!("Device manager error: {err}");
            }
        }
        Err(err) => {
            eprintln!("Failed to load schema: {err}");
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let rigs = load_rig_files("./rigs")?;

    let (gui_sender, gui_receiver) = mpsc::channel::<GuiMessage>(10);
    let device_manager = DeviceManager::new(rigs.clone());

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
                rigs.keys().cloned().collect(),
            )))
        }),
    )
    .unwrap();

    Ok(())
}
