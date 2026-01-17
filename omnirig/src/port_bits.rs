#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::sync::RwLock;
use windows::core::implement;
use windows::Win32::System::Com::{IDispatch, IDispatch_Impl};
use windows_core::HRESULT;

use auto_dispatch::auto_dispatch;

#[derive(Default)]
#[implement(IDispatch)]
pub struct PortBits {
    rts: RwLock<bool>,
    dtr: RwLock<bool>,
    cts: RwLock<bool>,
    dsr: RwLock<bool>,
    locked: RwLock<bool>,
}

#[auto_dispatch]
impl PortBits {
    #[id(0x01)]
    fn Lock(&self) -> Result<bool, HRESULT> {
        println!("PortBits::Lock() called");
        *self.locked.write().unwrap() = true;
        Ok(true)
    }

    #[id(0x02)]
    #[getter]
    fn Rts(&self) -> Result<bool, HRESULT> {
        println!("PortBits::Rts getter called");
        Ok(*self.rts.read().unwrap())
    }

    #[id(0x02)]
    #[setter]
    fn Rts(&self, value: bool) -> Result<(), HRESULT> {
        println!("PortBits::Rts setter called with value: {}", value);
        *self.rts.write().unwrap() = value;
        Ok(())
    }

    #[id(0x03)]
    #[getter]
    fn Dtr(&self) -> Result<bool, HRESULT> {
        println!("PortBits::Dtr getter called");
        Ok(*self.dtr.read().unwrap())
    }

    #[id(0x03)]
    #[setter]
    fn Dtr(&self, value: bool) -> Result<(), HRESULT> {
        println!("PortBits::Dtr setter called with value: {}", value);
        *self.dtr.write().unwrap() = value;
        Ok(())
    }

    #[id(0x04)]
    #[getter]
    fn Cts(&self) -> Result<bool, HRESULT> {
        println!("PortBits::Cts getter called");
        Ok(*self.cts.read().unwrap())
    }

    #[id(0x05)]
    #[getter]
    fn Dsr(&self) -> Result<bool, HRESULT> {
        println!("PortBits::Dsr getter called");
        Ok(*self.dsr.read().unwrap())
    }

    #[id(0x06)]
    fn Unlock(&self) -> Result<(), HRESULT> {
        println!("PortBits::Unlock() called");
        *self.locked.write().unwrap() = false;
        Ok(())
    }
}
