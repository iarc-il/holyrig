#![allow(non_camel_case_types)]

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::{BSTR, GUID, IUnknown, PCWSTR, Result, implement};

use windows::Win32::Foundation::{
    CLASS_E_NOAGGREGATION, DISP_E_MEMBERNOTFOUND, DISP_E_PARAMNOTFOUND, E_INVALIDARG,
    E_NOINTERFACE, E_NOTIMPL, HWND,
};
use windows::Win32::System::Com::{
    CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED, CoInitializeEx, CoRegisterClassObject,
    CoRevokeClassObject, CoUninitialize, DISPATCH_FLAGS, DISPATCH_METHOD, DISPATCH_PROPERTYGET,
    DISPATCH_PROPERTYPUT, DISPPARAMS, EXCEPINFO, IClassFactory, IClassFactory_Impl, IDispatch,
    IDispatch_Impl, ITypeInfo, REGCLS_MULTIPLEUSE,
};

use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, TranslateMessage,
};
use windows_core::{BOOL, Error};

use auto_dispatch::auto_dispatch;

const CLSID_SIMPLE_COM_OBJECT: GUID = GUID::from_u128(0x12345678_1234_1234_1234_123456789ABC);

const IID_IUNKNOWN: GUID = GUID::from_u128(0x00000000_0000_0000_C000_000000000046);
const IID_IDISPATCH: GUID = GUID::from_u128(0x00020400_0000_0000_C000_000000000046);

const DISPID_COUNTER: i32 = 1;
const DISPID_GETMESSAGE: i32 = 2;
const DISPID_ADD: i32 = 3;

#[implement(IDispatch)]
struct SimpleComObject {
    counter: RefCell<i32>,
}

#[auto_dispatch]
impl SimpleComObject {
    // Getter for simple value
    #[id(1)]
    fn prop1(&self) -> Result<u32, HRESULT> {
        0
    }

    // Getter for other com component
    #[id(2)]
    fn other_com1(&self) -> Result<ComObj, HRESULT> {
        todo!()
    }

    // Getter
    #[id(3)]
    fn prop2(&self) -> Result<bool, HRESULT> {
        todo!()
    }

    // Setter
    #[id(3)]
    fn prop2(&self, value: bool) -> Result<(), HRESULT> {
        todo!()
    }
}

impl IDispatch_Impl for SimpleComObject_Impl {
    fn GetTypeInfoCount(&self) -> Result<u32> {
        Ok(0)
    }

    fn GetTypeInfo(&self, _itinfo: u32, _lcid: u32) -> Result<ITypeInfo> {
        Err(E_NOTIMPL.into())
    }

    fn GetIDsOfNames(
        &self,
        _riid: *const GUID,
        rgsznames: *const PCWSTR,
        cnames: u32,
        _lcid: u32,
        rgdispid: *mut i32,
    ) -> Result<()> {
        unsafe {
            if rgsznames.is_null() || rgdispid.is_null() {
                return Err(E_INVALIDARG.into());
            }

            for i in 0..cnames {
                let name_ptr = *rgsznames.add(i as usize);
                let name = name_ptr.to_string().unwrap_or_default().to_uppercase();

                let dispid = match name.as_str() {
                    "COUNTER" => DISPID_COUNTER,
                    "GETMESSAGE" => DISPID_GETMESSAGE,
                    "ADD" => DISPID_ADD,
                    _ => return Err(DISP_E_MEMBERNOTFOUND.into()),
                };

                *rgdispid.add(i as usize) = dispid;
            }

            Ok(())
        }
    }

    fn Invoke(
        &self,
        dispidmember: i32,
        _riid: *const GUID,
        _lcid: u32,
        wflags: DISPATCH_FLAGS,
        pdispparams: *const DISPPARAMS,
        pvarresult: *mut VARIANT,
        _pexcepinfo: *mut EXCEPINFO,
        _puargerr: *mut u32,
    ) -> Result<()> {
        unsafe {
            match dispidmember {
                DISPID_COUNTER => {
                    if wflags.contains(DISPATCH_PROPERTYGET) {
                        if !pvarresult.is_null() {
                            let value = *self.counter.borrow();
                            *pvarresult = value.into();
                        }
                        Ok(())
                    } else if wflags.contains(DISPATCH_PROPERTYPUT) {
                        if pdispparams.is_null() {
                            return Err(E_INVALIDARG.into());
                        }
                        let params = &*pdispparams;
                        if params.cArgs == 0 || params.rgvarg.is_null() {
                            return Err(DISP_E_PARAMNOTFOUND.into());
                        }
                        let value = &*params.rgvarg;

                        *self.counter.borrow_mut() = value
                            .try_into()
                            .or(Err(Error::from_hresult(E_INVALIDARG)))?;
                        Ok(())
                    } else {
                        Err(E_INVALIDARG.into())
                    }
                }
                DISPID_GETMESSAGE => {
                    if wflags.contains(DISPATCH_METHOD) {
                        if !pvarresult.is_null() {
                            *pvarresult = VARIANT::from(BSTR::from("Hello world"));
                        }
                        Ok(())
                    } else {
                        Err(E_INVALIDARG.into())
                    }
                }
                DISPID_ADD => {
                    if wflags.contains(DISPATCH_METHOD) {
                        if pdispparams.is_null() {
                            return Err(E_INVALIDARG.into());
                        }
                        let params = &*pdispparams;
                        if params.cArgs < 2 || params.rgvarg.is_null() {
                            return Err(DISP_E_PARAMNOTFOUND.into());
                        }
                        let arg2 = &*params.rgvarg.add(0);
                        let arg1 = &*params.rgvarg.add(1);

                        let val1: i32 =
                            arg1.try_into().or(Err(Error::from_hresult(E_INVALIDARG)))?;
                        let val2: i32 =
                            arg2.try_into().or(Err(Error::from_hresult(E_INVALIDARG)))?;

                        let sum = val1 + val2;

                        if !pvarresult.is_null() {
                            *pvarresult = sum.into();
                        }

                        Ok(())
                    } else {
                        Err(E_INVALIDARG.into())
                    }
                }
                _ => Err(DISP_E_MEMBERNOTFOUND.into()),
            }
        }
    }
}

#[implement(IClassFactory)]
struct SimpleComObjectFactory;

impl IClassFactory_Impl for SimpleComObjectFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: windows_core::Ref<IUnknown>,
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

            let instance: IDispatch = SimpleComObject {
                counter: RefCell::new(0),
            }
            .into();
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
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

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
