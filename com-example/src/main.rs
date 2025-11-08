#![allow(non_camel_case_types)]

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::core::{implement, IUnknown, Result, GUID, PCWSTR};

use windows::Win32::Foundation::{BOOL, CLASS_E_NOAGGREGATION, E_NOINTERFACE, E_NOTIMPL, HWND};
use windows::Win32::System::Com::{
    CoInitializeEx, CoRegisterClassObject, CoRevokeClassObject, CoUninitialize, IClassFactory,
    IClassFactory_Impl, IDispatch, IDispatch_Impl, ITypeInfo, CLSCTX_LOCAL_SERVER,
    COINIT_MULTITHREADED, DISPATCH_FLAGS, DISPPARAMS, EXCEPINFO, REGCLS_MULTIPLEUSE,
};

use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, MSG,
};

const CLSID_SIMPLE_COM_OBJECT: GUID = GUID::from_u128(0x12345678_1234_1234_1234_123456789ABC);

const IID_IUNKNOWN: GUID = GUID::from_u128(0x00000000_0000_0000_C000_000000000046);
const IID_IDISPATCH: GUID = GUID::from_u128(0x00020400_0000_0000_C000_000000000046);

#[implement(IDispatch)]
struct SimpleComObject;

impl IDispatch_Impl for SimpleComObject {
    fn GetTypeInfoCount(&self) -> Result<u32> {
        Ok(0)
    }

    fn GetTypeInfo(&self, _itinfo: u32, _lcid: u32) -> Result<ITypeInfo> {
        Err(E_NOTIMPL.into())
    }

    fn GetIDsOfNames(
        &self,
        _riid: *const GUID,
        _rgsznames: *const PCWSTR,
        _cnames: u32,
        _lcid: u32,
        _rgdispid: *mut i32,
    ) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn Invoke(
        &self,
        _dispidmember: i32,
        _riid: *const GUID,
        _lcid: u32,
        _wflags: DISPATCH_FLAGS,
        _pdispparams: *const DISPPARAMS,
        _pvarresult: *mut VARIANT,
        _pexcepinfo: *mut EXCEPINFO,
        _puargerr: *mut u32,
    ) -> Result<()> {
        Err(E_NOTIMPL.into())
    }
}

#[implement(IClassFactory)]
struct SimpleComObjectFactory;

impl IClassFactory_Impl for SimpleComObjectFactory {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> Result<()> {
        if punkouter.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }

        unsafe {
            let requested_iid = *riid;

            if requested_iid != IID_IUNKNOWN && requested_iid != IID_IDISPATCH {
                *ppvobject = std::ptr::null_mut();
                return Err(E_NOINTERFACE.into());
            }

            let instance: IDispatch = SimpleComObject.into();
            *ppvobject = std::mem::transmute_copy(&instance);
            std::mem::forget(instance);
        }
        Ok(())
    }

    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        Ok(())
    }
}

fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;

        let factory: IClassFactory = SimpleComObjectFactory.into();

        let cookie = CoRegisterClassObject(
            &CLSID_SIMPLE_COM_OBJECT,
            &factory,
            CLSCTX_LOCAL_SERVER,
            REGCLS_MULTIPLEUSE,
        )?;

        println!("COM server started successfully!");
        println!(
            "CLSID: {{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
            CLSID_SIMPLE_COM_OBJECT.data1,
            CLSID_SIMPLE_COM_OBJECT.data2,
            CLSID_SIMPLE_COM_OBJECT.data3,
            CLSID_SIMPLE_COM_OBJECT.data4[0],
            CLSID_SIMPLE_COM_OBJECT.data4[1],
            CLSID_SIMPLE_COM_OBJECT.data4[2],
            CLSID_SIMPLE_COM_OBJECT.data4[3],
            CLSID_SIMPLE_COM_OBJECT.data4[4],
            CLSID_SIMPLE_COM_OBJECT.data4[5],
            CLSID_SIMPLE_COM_OBJECT.data4[6],
            CLSID_SIMPLE_COM_OBJECT.data4[7],
        );
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
            let result = GetMessageW(&mut msg, HWND::default(), 0, 0);
            if result.0 == 0 || result.0 == -1 {
                break;
            }
            TranslateMessage(&msg);
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
