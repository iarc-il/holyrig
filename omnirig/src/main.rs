use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::core::GUID;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CoInitializeEx, CoRegisterClassObject, CoRevokeClassObject, CoUninitialize, IClassFactory,
    CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED, REGCLS_MULTIPLEUSE,
};
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, TranslateMessage, MSG};

use crate::omnirig::OmniRigXFactory;

mod enums;
mod omnirig;
mod port_bits;
mod rig;

const CLSID_OMNIRIG: GUID = GUID::from_u128(0x0839E8C6_ED30_4950_8087_966F970F0CAE);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

        let factory: IClassFactory = OmniRigXFactory.into();

        let cookie = CoRegisterClassObject(
            &CLSID_OMNIRIG,
            &factory,
            CLSCTX_LOCAL_SERVER,
            REGCLS_MULTIPLEUSE,
        )?;

        println!("OmniRig COM server started successfully!");
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
            let result = GetMessageW(&mut msg, Some(HWND::default()), 0, 0);
            if result.0 == 0 || result.0 == -1 {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);

            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        println!("Revoking class object...");
        CoRevokeClassObject(cookie)?;
        CoUninitialize();
        println!("Server stopped.");
    }

    Ok(())
}
