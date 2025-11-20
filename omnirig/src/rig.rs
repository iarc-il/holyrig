#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::sync::RwLock;
use windows::core::implement;
use windows::Win32::System::Com::{IDispatch, IDispatch_Impl};

use crate::enums::{RigParamX, RigStatusX};
use auto_dispatch::auto_dispatch;

#[derive(Default)]
#[implement(IDispatch)]
pub struct RigX {
    freq: RwLock<i32>,
    freq_a: RwLock<i32>,
    freq_b: RwLock<i32>,
    rit_offset: RwLock<i32>,
    pitch: RwLock<i32>,
    vfo: RwLock<RigParamX>,
    split: RwLock<RigParamX>,
    rit: RwLock<RigParamX>,
    xit: RwLock<RigParamX>,
    tx: RwLock<RigParamX>,
    mode: RwLock<RigParamX>,
    status: RwLock<RigStatusX>,
}

#[auto_dispatch]
impl RigX {
    #[id(0x08)]
    #[getter]
    fn Freq(&self) -> Result<i32, HRESULT> {
        println!("RigX::Freq getter called");
        Ok(*self.freq.read().unwrap())
    }

    #[id(0x08)]
    #[setter]
    fn Freq(&self, value: i32) -> Result<(), HRESULT> {
        println!("RigX::Freq setter called with value: {}", value);
        *self.freq.write().unwrap() = value;
        Ok(())
    }

    #[id(0x09)]
    #[getter]
    fn FreqA(&self) -> Result<i32, HRESULT> {
        println!("RigX::FreqA getter called");
        Ok(*self.freq_a.read().unwrap())
    }

    #[id(0x09)]
    #[setter]
    fn FreqA(&self, value: i32) -> Result<(), HRESULT> {
        println!("RigX::FreqA setter called with value: {}", value);
        *self.freq_a.write().unwrap() = value;
        Ok(())
    }

    #[id(0x0A)]
    #[getter]
    fn FreqB(&self) -> Result<i32, HRESULT> {
        println!("RigX::FreqB getter called");
        Ok(*self.freq_b.read().unwrap())
    }

    #[id(0x0A)]
    #[setter]
    fn FreqB(&self, value: i32) -> Result<(), HRESULT> {
        println!("RigX::FreqB setter called with value: {}", value);
        *self.freq_b.write().unwrap() = value;
        Ok(())
    }

    #[id(0x0B)]
    #[getter]
    fn RitOffset(&self) -> Result<i32, HRESULT> {
        println!("RigX::RitOffset getter called");
        Ok(*self.rit_offset.read().unwrap())
    }

    #[id(0x0B)]
    #[setter]
    fn RitOffset(&self, value: i32) -> Result<(), HRESULT> {
        println!("RigX::RitOffset setter called with value: {}", value);
        *self.rit_offset.write().unwrap() = value;
        Ok(())
    }

    #[id(0x0C)]
    #[getter]
    fn Pitch(&self) -> Result<i32, HRESULT> {
        println!("RigX::Pitch getter called");
        Ok(*self.pitch.read().unwrap())
    }

    #[id(0x0C)]
    #[setter]
    fn Pitch(&self, value: i32) -> Result<(), HRESULT> {
        println!("RigX::Pitch setter called with value: {}", value);
        *self.pitch.write().unwrap() = value;
        Ok(())
    }

    #[id(0x0D)]
    #[getter]
    fn Vfo(&self) -> Result<i32, HRESULT> {
        println!("RigX::Vfo getter called");
        let vfo = *self.vfo.read().unwrap();
        Ok(vfo.into())
    }

    #[id(0x0D)]
    #[setter]
    fn Vfo(&self, value: i32) -> Result<(), HRESULT> {
        println!("RigX::Vfo setter called with value: {}", value);
        *self.vfo.write().unwrap() = RigParamX::from(value);
        Ok(())
    }

    #[id(0x0E)]
    #[getter]
    fn Split(&self) -> Result<i32, HRESULT> {
        println!("RigX::Split getter called");
        let split = *self.split.read().unwrap();
        Ok(split.into())
    }

    #[id(0x0E)]
    #[setter]
    fn Split(&self, value: i32) -> Result<(), HRESULT> {
        println!("RigX::Split setter called with value: {}", value);
        *self.split.write().unwrap() = RigParamX::from(value);
        Ok(())
    }

    #[id(0x0F)]
    #[getter]
    fn Rit(&self) -> Result<i32, HRESULT> {
        println!("RigX::Rit getter called");
        let rit = *self.rit.read().unwrap();
        Ok(rit.into())
    }

    #[id(0x0F)]
    #[setter]
    fn Rit(&self, value: i32) -> Result<(), HRESULT> {
        println!("RigX::Rit setter called with value: {}", value);
        *self.rit.write().unwrap() = RigParamX::from(value);
        Ok(())
    }

    #[id(0x10)]
    #[getter]
    fn Xit(&self) -> Result<i32, HRESULT> {
        println!("RigX::Xit getter called");
        let xit = *self.xit.read().unwrap();
        Ok(xit.into())
    }

    #[id(0x10)]
    #[setter]
    fn Xit(&self, value: i32) -> Result<(), HRESULT> {
        println!("RigX::Xit setter called with value: {}", value);
        *self.xit.write().unwrap() = RigParamX::from(value);
        Ok(())
    }

    #[id(0x11)]
    #[getter]
    fn Tx(&self) -> Result<i32, HRESULT> {
        println!("RigX::Tx getter called");
        let tx = *self.tx.read().unwrap();
        Ok(tx.into())
    }

    #[id(0x11)]
    #[setter]
    fn Tx(&self, value: i32) -> Result<(), HRESULT> {
        println!("RigX::Tx setter called with value: {}", value);
        *self.tx.write().unwrap() = RigParamX::from(value);
        Ok(())
    }

    #[id(0x12)]
    #[getter]
    fn Mode(&self) -> Result<i32, HRESULT> {
        println!("RigX::Mode getter called");
        let mode = *self.mode.read().unwrap();
        Ok(mode.into())
    }

    #[id(0x12)]
    #[setter]
    fn Mode(&self, value: i32) -> Result<(), HRESULT> {
        println!("RigX::Mode setter called with value: {}", value);
        *self.mode.write().unwrap() = RigParamX::from(value);
        Ok(())
    }

    #[id(0x06)]
    #[getter]
    fn Status(&self) -> Result<i32, HRESULT> {
        println!("RigX::Status getter called");
        let status = *self.status.read().unwrap();
        Ok(status.into())
    }
}
