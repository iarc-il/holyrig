#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::sync::RwLock;
use windows::Win32::System::Com::{IDispatch, IDispatch_Impl};
use windows::core::implement;

use auto_dispatch::auto_dispatch;

#[derive(Default)]
#[implement(IDispatch)]
pub struct RigX {
    freq: RwLock<i32>,
    freq_a: RwLock<i32>,
    freq_b: RwLock<i32>,
    rit_offset: RwLock<i32>,
    pitch: RwLock<i32>,
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
}
