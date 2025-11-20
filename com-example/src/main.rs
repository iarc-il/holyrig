#![allow(non_camel_case_types)]

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::{GUID, IUnknown, implement};

use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_NOINTERFACE, HWND};
use windows::Win32::System::Com::{
    CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED, CoInitializeEx, CoRegisterClassObject,
    CoRevokeClassObject, CoUninitialize, IClassFactory, IClassFactory_Impl, IDispatch,
    IDispatch_Impl, REGCLS_MULTIPLEUSE,
};

use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, TranslateMessage,
};
use windows_core::{Interface, BOOL};

use auto_dispatch::auto_dispatch;

const CLSID_SIMPLE_COM_OBJECT: GUID = GUID::from_u128(0x12345678_1234_1234_1234_123456789ABC);

#[implement(IDispatch)]
struct SimpleObject {
    counter: RefCell<u32>,
    flag: RefCell<bool>,
}

#[auto_dispatch]
impl SimpleObject {
    #[id(1)]
    #[getter]
    fn prop1(&self) -> Result<u32, HRESULT> {
        Ok(*self.counter.borrow())
    }

    #[id(2)]
    #[getter]
    fn prop2(&self) -> Result<bool, HRESULT> {
        Ok(*self.flag.borrow())
    }

    #[id(2)]
    #[setter]
    fn prop2(&self, value: bool) -> Result<(), HRESULT> {
        *self.flag.borrow_mut() = value;
        Ok(())
    }

    #[id(3)]
    fn do_stuff(&self, value1: bool, value2: u16, value3: u16) -> Result<(), HRESULT> {
        println!("Value1: {value1}");
        println!("Value2: {value2}");
        println!("Value3: {value3}");
        if value1 {
            println!("Result: {}", value2 * value3);
        } else {
            println!("Result: {}", value2 + value3);
        }
        Ok(())
    }

    #[id(4)]
    fn do_other_stuff(&self, value1: bool, value3: i64) -> Result<u32, HRESULT> {
        if value1 {
            Ok((value3 * 3 + 1) as u32)
        } else {
            Ok((value3 / 2) as u32)
        }
    }
}

#[implement(IClassFactory)]
struct SimpleObjectFactory;

impl IClassFactory_Impl for SimpleObjectFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: windows_core::Ref<IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> windows::core::Result<()> {
        if punkouter.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }

        unsafe {
            let requested_iid = *riid;

            if requested_iid != IUnknown::IID && requested_iid != IDispatch::IID {
                *ppvobject = std::ptr::null_mut();
                return Err(E_NOINTERFACE.into());
            }

            let instance: IDispatch = SimpleObject {
                counter: RefCell::new(0),
                flag: RefCell::new(false),
            }
            .into();
            *ppvobject = std::mem::transmute_copy(&instance);
            std::mem::forget(instance);
        }
        Ok(())
    }

    fn LockServer(&self, _flock: BOOL) -> windows::core::Result<()> {
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

        let factory: IClassFactory = SimpleObjectFactory.into();

        let cookie = CoRegisterClassObject(
            &CLSID_SIMPLE_COM_OBJECT,
            &factory,
            CLSCTX_LOCAL_SERVER,
            REGCLS_MULTIPLEUSE,
        )?;

        println!("COM server started successfully!");
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
