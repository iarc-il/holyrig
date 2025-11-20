#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::sync::RwLock;
use windows::core::implement;
use windows::Win32::System::Com::{IDispatch, IDispatch_Impl};

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
