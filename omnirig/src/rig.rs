#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::sync::RwLock;
use windows::core::{implement, BSTR};
use windows::Win32::System::Com::{IDispatch, IDispatch_Impl, IDispatch_Vtbl};
use windows_core::{interface, HRESULT};

use crate::enums::{RigParamX, RigStatusX};
use crate::port_bits::PortBits;
use auto_dispatch::auto_dispatch;

#[interface("D30A7E51-5862-45B7-BFFA-6415917DA0CF")]
pub unsafe trait IRigX: IDispatch {
    fn get_RigType(&self, Value: *mut BSTR) -> HRESULT;
    fn get_ReadableParams(&self, Value: *mut i32) -> HRESULT;
    fn get_WriteableParams(&self, Value: *mut i32) -> HRESULT;
    fn IsParamReadable(&self, Param: i32, Value: *mut bool) -> HRESULT;
    fn IsParamWriteable(&self, Param: i32, Value: *mut bool) -> HRESULT;
    fn get_Status(&self, Value: *mut i32) -> HRESULT;
    fn get_StatusStr(&self, Value: *mut BSTR) -> HRESULT;
    fn get_Freq(&self, Value: *mut i32) -> HRESULT;
    fn put_Freq(&self, Value: i32) -> HRESULT;
    fn get_FreqA(&self, Value: *mut i32) -> HRESULT;
    fn put_FreqA(&self, Value: i32) -> HRESULT;
    fn get_FreqB(&self, Value: *mut i32) -> HRESULT;
    fn put_FreqB(&self, Value: i32) -> HRESULT;
    fn get_RitOffset(&self, Value: *mut i32) -> HRESULT;
    fn put_RitOffset(&self, Value: i32) -> HRESULT;
    fn get_Pitch(&self, Value: *mut i32) -> HRESULT;
    fn put_Pitch(&self, Value: i32) -> HRESULT;
    fn get_Vfo(&self, Value: *mut i32) -> HRESULT;
    fn put_Vfo(&self, Value: i32) -> HRESULT;
    fn get_Split(&self, Value: *mut i32) -> HRESULT;
    fn put_Split(&self, Value: i32) -> HRESULT;
    fn get_Rit(&self, Value: *mut i32) -> HRESULT;
    fn put_Rit(&self, Value: i32) -> HRESULT;
    fn get_Xit(&self, Value: *mut i32) -> HRESULT;
    fn put_Xit(&self, Value: i32) -> HRESULT;
    fn get_Tx(&self, Value: *mut i32) -> HRESULT;
    fn put_Tx(&self, Value: i32) -> HRESULT;
    fn get_Mode(&self, Value: *mut i32) -> HRESULT;
    fn put_Mode(&self, Value: i32) -> HRESULT;
    fn ClearRit(&self) -> HRESULT;
    fn SetSimplexMode(&self, Freq: i32) -> HRESULT;
    fn SetSplitMode(&self, RxFreq: i32, TxFreq: i32) -> HRESULT;
    fn FrequencyOfTone(&self, Tone: i32, Value: *mut i32) -> HRESULT;
    // TODO: SendCustomCommand requires VARIANT support in auto_dispatch
    // fn SendCustomCommand(&self, Command: VARIANT, ReplyLength: i32, ReplyEnd: VARIANT) -> HRESULT;
    fn GetRxFrequency(&self, Value: *mut i32) -> HRESULT;
    fn GetTxFrequency(&self, Value: *mut i32) -> HRESULT;
    fn get_PortBits(&self, Value: *mut Option<IDispatch>) -> HRESULT;
}

#[implement(IDispatch)]
pub struct RigX {
    rig_type: RwLock<String>,
    status_str: RwLock<String>,
    readable_params: RwLock<i32>,
    writeable_params: RwLock<i32>,
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
    port_bits: RwLock<Option<IDispatch>>,
}

impl Default for RigX {
    fn default() -> Self {
        let port_bits: IDispatch = PortBits::default().into();

        Self {
            rig_type: RwLock::new("DummyRig".to_string()),
            status_str: RwLock::new("Not configured".to_string()),
            readable_params: RwLock::new(0),
            writeable_params: RwLock::new(0),
            freq: RwLock::new(0),
            freq_a: RwLock::new(0),
            freq_b: RwLock::new(0),
            rit_offset: RwLock::new(0),
            pitch: RwLock::new(0),
            vfo: RwLock::new(RigParamX::default()),
            split: RwLock::new(RigParamX::default()),
            rit: RwLock::new(RigParamX::default()),
            xit: RwLock::new(RigParamX::default()),
            tx: RwLock::new(RigParamX::default()),
            mode: RwLock::new(RigParamX::default()),
            status: RwLock::new(RigStatusX::default()),
            port_bits: RwLock::new(Some(port_bits)),
        }
    }
}

#[auto_dispatch]
impl RigX {
    #[id(0x01)]
    #[getter]
    fn RigType(&self) -> Result<BSTR, HRESULT> {
        println!("RigX::RigType getter called");
        let rig_type = self.rig_type.read().unwrap();
        Ok(BSTR::from(rig_type.as_str()))
    }

    #[id(0x02)]
    #[getter]
    fn ReadableParams(&self) -> Result<i32, HRESULT> {
        println!("RigX::ReadableParams getter called");
        Ok(*self.readable_params.read().unwrap())
    }

    #[id(0x03)]
    #[getter]
    fn WriteableParams(&self) -> Result<i32, HRESULT> {
        println!("RigX::WriteableParams getter called");
        Ok(*self.writeable_params.read().unwrap())
    }

    #[id(0x04)]
    fn IsParamReadable(&self, param: i32) -> Result<bool, HRESULT> {
        println!("RigX::IsParamReadable called with param: {}", param);
        let readable_params = *self.readable_params.read().unwrap();
        Ok((readable_params & param) != 0)
    }

    #[id(0x05)]
    fn IsParamWriteable(&self, param: i32) -> Result<bool, HRESULT> {
        println!("RigX::IsParamWriteable called with param: {}", param);
        let writeable_params = *self.writeable_params.read().unwrap();
        Ok((writeable_params & param) != 0)
    }

    #[id(0x07)]
    #[getter]
    fn StatusStr(&self) -> Result<BSTR, HRESULT> {
        println!("RigX::StatusStr getter called");
        let status_str = self.status_str.read().unwrap();
        Ok(BSTR::from(status_str.as_str()))
    }

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

    #[id(0x13)]
    fn ClearRit(&self) -> Result<(), HRESULT> {
        println!("RigX::ClearRit called");
        *self.rit_offset.write().unwrap() = 0;
        Ok(())
    }

    #[id(0x14)]
    fn SetSimplexMode(&self, freq: i32) -> Result<(), HRESULT> {
        println!("RigX::SetSimplexMode called with freq: {}", freq);
        *self.freq.write().unwrap() = freq;
        *self.freq_a.write().unwrap() = freq;
        *self.freq_b.write().unwrap() = freq;
        *self.split.write().unwrap() = RigParamX::SplitOff;
        Ok(())
    }

    #[id(0x15)]
    fn SetSplitMode(&self, rx_freq: i32, tx_freq: i32) -> Result<(), HRESULT> {
        println!(
            "RigX::SetSplitMode called with rx_freq: {}, tx_freq: {}",
            rx_freq, tx_freq
        );
        *self.freq_a.write().unwrap() = rx_freq;
        *self.freq_b.write().unwrap() = tx_freq;
        *self.split.write().unwrap() = RigParamX::SplitOn;
        Ok(())
    }

    #[id(0x16)]
    fn FrequencyOfTone(&self, tone: i32) -> Result<i32, HRESULT> {
        println!("RigX::FrequencyOfTone called with tone: {}", tone);
        Ok(tone * 10)
    }

    // TODO: auto_dispatch doesn't support VARIANT parameters yet
    // #[id(0x17)]
    // fn SendCustomCommand(
    //     &self,
    //     command: VARIANT,
    //     reply_length: i32,
    //     reply_end: VARIANT,
    // ) -> Result<(), HRESULT> {
    //     println!(
    //         "RigX::SendCustomCommand called with reply_length: {}",
    //         reply_length
    //     );
    //     Ok(())
    // }

    #[id(0x18)]
    fn GetRxFrequency(&self) -> Result<i32, HRESULT> {
        println!("RigX::GetRxFrequency called");
        Ok(*self.freq_a.read().unwrap())
    }

    #[id(0x19)]
    fn GetTxFrequency(&self) -> Result<i32, HRESULT> {
        println!("RigX::GetTxFrequency called");
        let split = *self.split.read().unwrap();
        if split == RigParamX::SplitOn {
            Ok(*self.freq_b.read().unwrap())
        } else {
            Ok(*self.freq_a.read().unwrap())
        }
    }

    #[id(0x1A)]
    #[getter]
    fn PortBits(&self) -> Result<IDispatch, HRESULT> {
        println!("RigX::PortBits getter called");
        self.port_bits
            .read()
            .unwrap()
            .as_ref()
            .cloned()
            .ok_or(windows::Win32::Foundation::E_FAIL)
    }
}
