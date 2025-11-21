#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::c_void;
use std::sync::RwLock;
use windows::core::{implement, IUnknown, Interface, GUID};
use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_NOINTERFACE};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl, IDispatch, IDispatch_Impl};
use windows_core::BOOL;

use crate::rig::RigX;
use auto_dispatch::auto_dispatch;

#[implement(IDispatch)]
pub struct OmniRigX {
    interface_version: RwLock<i32>,
    software_version: RwLock<i32>,
    dialog_visible: RwLock<bool>,
    rig1: RwLock<Option<IDispatch>>,
    rig2: RwLock<Option<IDispatch>>,
}

impl Default for OmniRigX {
    fn default() -> Self {
        let rig1: IDispatch = RigX::default().into();
        let rig2: IDispatch = RigX::default().into();

        Self {
            interface_version: RwLock::new(0x101),
            software_version: RwLock::new(0x101),
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
        Ok(*self.interface_version.read().unwrap())
    }

    #[id(0x02)]
    #[getter]
    fn SoftwareVersion(&self) -> Result<i32, HRESULT> {
        println!("OmniRigX::SoftwareVersion getter called");
        Ok(*self.software_version.read().unwrap())
    }

    #[id(0x03)]
    #[getter]
    fn Rig1(&self) -> Result<IDispatch, HRESULT> {
        println!("OmniRigX::Rig1 getter called");
        self.rig1
            .read()
            .unwrap()
            .as_ref()
            .cloned()
            .ok_or(windows::Win32::Foundation::E_FAIL)
    }

    #[id(0x04)]
    #[getter]
    fn Rig2(&self) -> Result<IDispatch, HRESULT> {
        println!("OmniRigX::Rig2 getter called");
        self.rig2
            .read()
            .unwrap()
            .as_ref()
            .cloned()
            .ok_or(windows::Win32::Foundation::E_FAIL)
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
            let instance: IDispatch = OmniRigX::default().into();
            *ppvobject = std::mem::transmute_copy(&instance);
            std::mem::forget(instance);
        }
        Ok(())
    }

    fn LockServer(&self, _flock: BOOL) -> windows::core::Result<()> {
        Ok(())
    }
}
