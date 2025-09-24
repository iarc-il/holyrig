use windows::core::*;


#[interface("12345678-1234-1234-1234-1234567890AB")]
pub unsafe trait IOmnirigInterface: IUnknown {
    unsafe fn set_freq(&self) -> Result<()>;
}

#[implement(IOmnirigInterface)]
struct Omnirig;

impl IOmnirigInterface_Impl for Omnirig_Impl {
    unsafe fn set_freq(&self) -> Result<()> {
        Ok(())
    }
}
