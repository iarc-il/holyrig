#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::c_void;
use std::sync::RwLock;
use windows::core::{implement, IUnknown, Interface, GUID};
use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_NOINTERFACE};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl, IDispatch, IDispatch_Impl};
use windows_core::{BOOL, HRESULT};

use crate::com_interface::IOmniRigX;
use crate::rig::{IRigX, RigX};
use auto_dispatch::auto_dispatch;

#[implement(IOmniRigX)]
pub struct OmniRigX {
    dialog_visible: RwLock<bool>,
    rig1: RwLock<Option<IRigX>>,
    rig2: RwLock<Option<IRigX>>,
}

impl Default for OmniRigX {
    fn default() -> Self {
        let rig1: IRigX = RigX::default().into();
        let rig2: IRigX = RigX::default().into();

        Self {
            dialog_visible: RwLock::new(false),
            rig1: RwLock::new(Some(rig1)),
            rig2: RwLock::new(Some(rig2)),
        }
    }
}

#[auto_dispatch]
impl OmniRigX {
    #[id(0x01)]
    #[getter]
    fn InterfaceVersion(&self) -> Result<i32, HRESULT> {
        println!("OmniRigX::InterfaceVersion getter called");
        Ok(0x101)
    }

    #[id(0x02)]
    #[getter]
    fn SoftwareVersion(&self) -> Result<i32, HRESULT> {
        println!("OmniRigX::SoftwareVersion getter called");
        Ok(0x10014)
    }

    #[id(0x03)]
    #[getter]
    fn Rig1(&self) -> Result<IDispatch, HRESULT> {
        println!("OmniRigX::Rig1 getter called");
        let rig = self
            .rig1
            .read()
            .unwrap()
            .as_ref()
            .cloned()
            .ok_or(windows::Win32::Foundation::E_FAIL)?;
        Ok(rig.cast()?)
    }

    #[id(0x04)]
    #[getter]
    fn Rig2(&self) -> Result<IDispatch, HRESULT> {
        println!("OmniRigX::Rig2 getter called");
        let rig = self
            .rig2
            .read()
            .unwrap()
            .as_ref()
            .cloned()
            .ok_or(windows::Win32::Foundation::E_FAIL)?;
        Ok(rig.cast()?)
    }

    #[id(0x05)]
    #[getter]
    fn DialogVisible(&self) -> Result<bool, HRESULT> {
        println!("OmniRigX::DialogVisible getter called");
        Ok(*self.dialog_visible.read().unwrap())
    }

    #[id(0x05)]
    #[setter]
    fn DialogVisible(&self, value: bool) -> Result<(), HRESULT> {
        println!(
            "OmniRigX::DialogVisible setter called with value: {}",
            value
        );
        *self.dialog_visible.write().unwrap() = value;
        Ok(())
    }
}

impl crate::com_interface::IOmniRigX_Impl for OmniRigX_Impl {
    unsafe fn get_InterfaceVersion(&self, value: *mut i32) -> HRESULT {
        *value = self.get_InterfaceVersion().unwrap();
        HRESULT(0)
    }
    unsafe fn get_SoftwareVersion(&self, value: *mut i32) -> HRESULT {
        *value = self.get_SoftwareVersion().unwrap();
        HRESULT(0)
    }
    unsafe fn get_Rig1(&self, value: *mut Option<IRigX>) -> HRESULT {
        let disp = self.get_Rig1().unwrap();
        *value = Some(disp.cast().unwrap());
        HRESULT(0)
    }
    unsafe fn get_Rig2(&self, value: *mut Option<IRigX>) -> HRESULT {
        let disp = self.get_Rig2().unwrap();
        *value = Some(disp.cast().unwrap());
        HRESULT(0)
    }
    unsafe fn get_DialogVisible(&self, value: *mut bool) -> HRESULT {
        *value = self.get_DialogVisible().unwrap();
        HRESULT(0)
    }
    unsafe fn set_DialogVisible(&self, value: bool) -> HRESULT {
        self.set_DialogVisible(value).unwrap();
        HRESULT(0)
    }
}

#[implement(IClassFactory)]
pub struct OmniRigXFactory;

impl IClassFactory_Impl for OmniRigXFactory_Impl {
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

            println!("OmniRigXFactory: Creating new OmniRigX instance");
            let instance: IOmniRigX = OmniRigX::default().into();
            *ppvobject = std::mem::transmute_copy(&instance);
            std::mem::forget(instance);
        }
        Ok(())
    }

    fn LockServer(&self, _flock: BOOL) -> windows::core::Result<()> {
        Ok(())
    }
}
