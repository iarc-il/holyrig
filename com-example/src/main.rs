use windows::core::{implement, Result, GUID, PCWSTR};

use windows::Win32::Foundation::E_NOTIMPL;
use windows::Win32::System::Com::{
    CoInitializeEx, CoUninitialize, IDispatch, IDispatch_Impl, ITypeInfo, COINIT_MULTITHREADED,
    DISPATCH_FLAGS, DISPPARAMS, EXCEPINFO,
};

use windows::Win32::System::Variant::VARIANT;

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

fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)?;

        let com_object = SimpleComObject;
        let _dispatch: IDispatch = com_object.into();

        println!("COM IDispatch server initialized successfully!");
        println!("SimpleComObject implements IDispatch with stub methods.");

        CoUninitialize();
    }

    Ok(())
}
