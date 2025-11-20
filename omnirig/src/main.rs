use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

mod enums;
mod port_bits;
mod rig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

        println!("OmniRig COM server initialized");
        println!("Press Ctrl+C to stop the server...");

        CoUninitialize();
        println!("Server stopped.");
    }

    Ok(())
}
