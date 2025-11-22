use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::core::GUID;
use windows::Win32::System::Com::{
    CoInitializeEx, CoRegisterClassObject, CoRevokeClassObject, CoUninitialize, IClassFactory,
    CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED, REGCLS_MULTIPLEUSE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
};

use crate::omnirig::OmniRigXFactory;

mod enums;
mod omnirig;
mod port_bits;
pub mod registry_backup;
mod rig;

const CLSID_OMNIRIG: GUID = GUID::from_u128(0x0839E8C6_ED30_4950_8087_966F970F0CAE);

pub fn run_omnirig_server() -> Result<(), Box<dyn std::error::Error>> {
    use registry_backup::*;

    if backup_exists() {
        println!("Warning! Backup already exists");
        let backup = load_backup()?;
        restore_omnirig_registry(&backup)?;
        delete_backup()?;
    }

    println!("\nBacking up original OmniRig registry...");
    let backup = backup_omnirig_registry()?;
    save_backup(&backup)?;

    if let Some(ref original_path) = backup.original_exe_path {
        println!("Original OmniRig path: {}", original_path);
    }

    let exe_path = std::env::current_exe()?;
    let exe_path_str = exe_path.to_str().ok_or("Invalid executable path")?;

    println!("\nRegistering HolyRig COM component...");
    register_holyrig_com_component(&CLSID_OMNIRIG, exe_path_str)?;

    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

        let factory: IClassFactory = OmniRigXFactory.into();

        let cookie = CoRegisterClassObject(
            &CLSID_OMNIRIG,
            &factory,
            CLSCTX_LOCAL_SERVER,
            REGCLS_MULTIPLEUSE,
        )?;

        println!("\nOmniRig COM server started successfully!");
        println!("CLSID: {{0839E8C6-ED30-4950-8087-966F970F0CAE}}");
        println!("Press Ctrl+C to stop the server...");

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        ctrlc::set_handler(move || {
            println!("\nReceived Ctrl+C, shutting down...");
            running_clone.store(false, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");

        let mut msg = MSG::default();
        while running.load(Ordering::SeqCst) {
            if PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            } else {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        println!("\nRevoking class object...");
        CoRevokeClassObject(cookie)?;
        CoUninitialize();

        println!("\nRestoring original OmniRig registry...");
        match restore_omnirig_registry(&backup) {
            Ok(_) => {
                delete_backup()?;
                println!("Original OmniRig restored successfully.");
            }
            Err(err) => {
                eprintln!("Warning: Failed to restore registry: {err}");
                eprintln!("Backup file preserved. Run with --status to check state.");
            }
        }

        println!("Server stopped.");
    }

    Ok(())
}
