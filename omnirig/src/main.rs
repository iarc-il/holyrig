use omnirig::registry_backup;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "--status" {
        show_status()
    } else {
        omnirig::run_omnirig_server()
    }
}

fn show_status() -> Result<(), Box<dyn std::error::Error>> {
    println!("OmniRig Status Report");
    println!("=====================\n");

    let backup_path = registry_backup::get_backup_file_path()?;
    let backup_exists = backup_path.exists();

    println!("Backup file: {}", backup_path.display());
    println!(
        "Backup exists: {}",
        if backup_exists { "YES" } else { "NO" }
    );

    if backup_exists {
        match registry_backup::load_backup() {
            Ok(backup) => {
                println!("Backup timestamp: {}", backup.backup_timestamp);
                if let Some(ref original_path) = backup.original_exe_path {
                    println!("Original OmniRig path: {}", original_path);
                } else {
                    println!("Original OmniRig path: Not installed");
                }
            }
            Err(err) => {
                println!("WARNING: Backup file exists but is corrupted: {err}");
            }
        }
        println!("\nWARNING: Backup exists - this may indicate a crash or improper shutdown.");
        println!("Run the server normally to restore or handle the backup.");
    }

    println!();

    match registry_backup::get_current_omnirig_path()? {
        Some(path) => {
            println!("Currently registered OmniRig: {}", path);
            let current_exe = std::env::current_exe()?;
            let current_exe_str = current_exe.to_str().unwrap_or("");

            if path.to_lowercase().contains("holyrig") || path == current_exe_str {
                println!("Status: HolyRig OmniRig is registered");
            } else {
                println!("Status: Original OmniRig is registered");
            }
        }
        None => {
            println!("Currently registered OmniRig: None");
            println!("Status: No OmniRig is registered in the system");
        }
    }

    Ok(())
}
