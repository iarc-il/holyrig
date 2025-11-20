use std::ffi::c_void;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use windows::core::{GUID, IUnknown, implement};

use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_FAIL, E_NOINTERFACE};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl, IDispatch, IDispatch_Impl};

use windows_core::{BOOL, Interface};

use auto_dispatch::auto_dispatch;

use crate::sub_object::SubObject;

struct SendSyncIDispatch(IDispatch);

unsafe impl Send for SendSyncIDispatch {}
unsafe impl Sync for SendSyncIDispatch {}

static SINGLETON: OnceLock<Arc<Mutex<Option<SendSyncIDispatch>>>> = OnceLock::new();

#[implement(IDispatch)]
struct SimpleObject {
    counter: RwLock<u32>,
    flag: RwLock<bool>,
    sub1: RwLock<Option<IDispatch>>,
    sub2: RwLock<Option<IDispatch>>,
}

impl Default for SimpleObject {
    fn default() -> Self {
        let sub1: IDispatch = SubObject::default().into();
        let sub2: IDispatch = SubObject::default().into();

        Self {
            counter: RwLock::new(0),
            flag: RwLock::new(false),
            sub1: RwLock::new(Some(sub1)),
            sub2: RwLock::new(Some(sub2)),
        }
    }
}

#[auto_dispatch]
impl SimpleObject {
    #[id(1)]
    #[getter]
    fn prop1(&self) -> Result<u32, HRESULT> {
        Ok(*self.counter.read().unwrap())
    }

    #[id(2)]
    #[getter]
    fn prop2(&self) -> Result<bool, HRESULT> {
        Ok(*self.flag.read().unwrap())
    }

    #[id(2)]
    #[setter]
    fn prop2(&self, value: bool) -> Result<(), HRESULT> {
        *self.flag.write().unwrap() = value;
        Ok(())
    }

    #[id(3)]
    fn do_stuff(&self, value1: bool, value2: u16, value3: u16) -> Result<(), HRESULT> {
        println!("Value1: {value1}");
        println!("Value2: {value2}");
        println!("Value3: {value3}");
        if value1 {
            println!("Result: {}", value2 * value3);
        } else {
            println!("Result: {}", value2 + value3);
        }
        Ok(())
    }

    #[id(4)]
    fn do_other_stuff(&self, value1: bool, value3: i64) -> Result<u32, HRESULT> {
        if value1 {
            Ok((value3 * 3 + 1) as u32)
        } else {
            Ok((value3 / 2) as u32)
        }
    }

    #[id(5)]
    #[getter]
    fn sub1(&self) -> Result<IDispatch, HRESULT> {
        self.sub1.read().unwrap().as_ref().cloned().ok_or(E_FAIL)
    }

    #[id(6)]
    #[getter]
    fn sub2(&self) -> Result<IDispatch, HRESULT> {
        self.sub2.read().unwrap().as_ref().cloned().ok_or(E_FAIL)
    }
}

#[implement(IClassFactory)]
pub struct SimpleObjectFactory;

impl IClassFactory_Impl for SimpleObjectFactory_Impl {
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

            let singleton = SINGLETON.get_or_init(|| {
                let obj: IDispatch = SimpleObject::default().into();
                Arc::new(Mutex::new(Some(SendSyncIDispatch(obj))))
            });

            let instance = singleton.lock().unwrap().as_ref().unwrap().0.clone();

            *ppvobject = std::mem::transmute_copy(&instance);
            std::mem::forget(instance);
        }
        Ok(())
    }

    fn LockServer(&self, _flock: BOOL) -> windows::core::Result<()> {
        Ok(())
    }
}
