use std::cell::RefCell;
use std::ffi::c_void;
use windows::core::{GUID, IUnknown, implement};

use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_FAIL, E_NOINTERFACE};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl, IDispatch, IDispatch_Impl};

use windows_core::{BOOL, Interface};

use auto_dispatch::auto_dispatch;

#[derive(Default)]
#[implement(IDispatch)]
pub struct SubObject {
    value: RefCell<u32>,
}

#[auto_dispatch]
impl SubObject {
    #[id(1)]
    #[getter]
    fn value(&self) -> Result<u32, HRESULT> {
        Ok(*self.value.borrow())
    }

    #[id(1)]
    #[setter]
    fn value(&self, value: u32) -> Result<(), HRESULT> {
        *self.value.borrow_mut() = value;
        Ok(())
    }

    #[id(2)]
    fn inc(&self) -> Result<(), HRESULT> {
        *self.value.borrow_mut() += 1;
        Ok(())
    }

    #[id(3)]
    fn dec(&self) -> Result<(), HRESULT> {
        if *self.value.borrow() > 0 {
            *self.value.borrow_mut() -= 1;
            Ok(())
        } else {
            Err(E_FAIL)
        }
    }
}

#[implement(IClassFactory)]
pub struct SubObjectFactory;

impl IClassFactory_Impl for SubObjectFactory_Impl {
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

            let instance: IDispatch = SubObject {
                value: RefCell::new(0),
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
