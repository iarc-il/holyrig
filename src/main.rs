use anyhow::Result;
use eframe::egui;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;

use holyrig::{
    Interpreter, SchemaFile, gui, parse_and_validate_with_schema, parse_schema, rigctld, serial,
    udp_server,
};

use gui::GuiMessage;
use serial::manager::DeviceManager;

fn load_rig_files<P: AsRef<Path>>(
    dir_path: P,
    schema: &SchemaFile,
) -> Result<Arc<HashMap<String, Interpreter>>> {
    let mut rigs = HashMap::new();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("rig") {
            continue;
        }

        let file_name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?
            .to_string();

        let content = std::fs::read_to_string(path)?;
        let rig_file = parse_and_validate_with_schema(&content, schema).unwrap();
        rigs.insert(file_name, Interpreter::new(rig_file));
    }

    Ok(Arc::new(rigs))
}

fn load_schema_file(base_dirs: &xdg::BaseDirectories) -> Result<SchemaFile> {
    let schema_path = if cfg!(debug_assertions) {
        std::path::PathBuf::from("schema/transceiver.schema")
    } else {
        base_dirs.place_config_file("schema.toml")?
    };
    Ok(parse_schema(&std::fs::read_to_string(schema_path)?)?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let base_dirs = xdg::BaseDirectories::with_prefix("holyrig")?;
    let schema = load_schema_file(&base_dirs)?;
    let rigs = load_rig_files("./rigs", &schema)?;

    let (gui_sender, gui_receiver) = mpsc::channel::<GuiMessage>(10);
    let mut device_manager: DeviceManager<Interpreter> =
        DeviceManager::new(rigs.clone(), base_dirs.clone());

    let gui_command_sender = device_manager.sender();
    let udp_command_sender = device_manager.sender();
    let rigctld_command_sender = device_manager.sender();
    let udp_message_receiver = device_manager.receiver();
    let rigctld_message_receiver = device_manager.receiver();

    tokio::spawn(async move { device_manager.run(gui_sender).await });

    tokio::spawn(async move {
        if let Err(err) = udp_server::run_server(udp_command_sender, udp_message_receiver).await {
            eprintln!("UDP server error: {err}");
        }
    });

    tokio::spawn(async move {
        if let Err(err) =
            rigctld::run_server(rigctld_command_sender, rigctld_message_receiver).await
        {
            eprintln!("Rigctld server error: {err}");
        }
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([350.0, 440.0])
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
