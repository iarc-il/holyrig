use anyhow::Result;
use eframe::egui;
use schema::Schema;
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

fn load_rig_files<P: AsRef<Path>>(
    dir_path: P,
    schema: &Schema,
) -> Result<Arc<HashMap<String, RigApi>>> {
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

        match RigApi::try_from((rig_file, schema.clone())) {
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
    let schema_path = if cfg!(debug_assertions) {
        std::path::PathBuf::from("./schema/transceiver.toml")
    } else {
        let xdg_dirs = xdg::BaseDirectories::with_prefix("holyrig")?;
        xdg_dirs.place_config_file("schema.toml")?
    };
    schema::Schema::load(schema_path).map_err(|err| anyhow::anyhow!("Failed to load schema: {err}"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let schema = load_schema_file()?;
    let rigs = load_rig_files("./rigs", &schema)?;

    let (gui_sender, gui_receiver) = mpsc::channel::<GuiMessage>(10);
    let mut device_manager = DeviceManager::new(rigs.clone());

    let gui_command_sender = device_manager.command_sender();
    let udp_command_sender = device_manager.command_sender();

    tokio::spawn(async move { device_manager.run(gui_sender).await });

    tokio::spawn(async move {
        if let Err(err) = udp_server::run_server(udp_command_sender, &schema).await {
            eprintln!("UDP server error: {err}");
        }
    });

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
                gui_command_sender,
                rigs.keys().cloned().collect(),
            )))
        }),
    )
    .unwrap();

    Ok(())
}
