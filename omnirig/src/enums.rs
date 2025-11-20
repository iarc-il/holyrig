#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RigParamX {
    #[default]
    Unknown = 1,
    Freq = 2,
    FreqA = 4,
    FreqB = 8,
    Pitch = 16,
    RitOffset = 32,
    Rit0 = 64,
    VfoAA = 128,
    VfoAB = 256,
    VfoBA = 512,
    VfoBB = 1024,
    VfoA = 2048,
    VfoB = 4096,
    VfoEqual = 8192,
    VfoSwap = 16384,
    SplitOn = 32768,
    SplitOff = 65536,
    RitOn = 131072,
    RitOff = 262144,
    XitOn = 524288,
    XitOff = 1048576,
    Rx = 2097152,
    Tx = 4194304,
    CwU = 8388608,
    CwL = 16777216,
    SsbU = 33554432,
    SsbL = 67108864,
    DigU = 134217728,
    DigL = 268435456,
    Am = 536870912,
    Fm = 1073741824,
}

impl From<i32> for RigParamX {
    fn from(value: i32) -> Self {
        match value {
            1 => RigParamX::Unknown,
            2 => RigParamX::Freq,
            4 => RigParamX::FreqA,
            8 => RigParamX::FreqB,
            16 => RigParamX::Pitch,
            32 => RigParamX::RitOffset,
            64 => RigParamX::Rit0,
            128 => RigParamX::VfoAA,
            256 => RigParamX::VfoAB,
            512 => RigParamX::VfoBA,
            1024 => RigParamX::VfoBB,
            2048 => RigParamX::VfoA,
            4096 => RigParamX::VfoB,
            8192 => RigParamX::VfoEqual,
            16384 => RigParamX::VfoSwap,
            32768 => RigParamX::SplitOn,
            65536 => RigParamX::SplitOff,
            131072 => RigParamX::RitOn,
            262144 => RigParamX::RitOff,
            524288 => RigParamX::XitOn,
            1048576 => RigParamX::XitOff,
            2097152 => RigParamX::Rx,
            4194304 => RigParamX::Tx,
            8388608 => RigParamX::CwU,
            16777216 => RigParamX::CwL,
            33554432 => RigParamX::SsbU,
            67108864 => RigParamX::SsbL,
            134217728 => RigParamX::DigU,
            268435456 => RigParamX::DigL,
            536870912 => RigParamX::Am,
            1073741824 => RigParamX::Fm,
            _ => RigParamX::Unknown,
        }
    }
}

impl From<RigParamX> for i32 {
    fn from(value: RigParamX) -> Self {
        value as i32
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RigStatusX {
    #[default]
    NotConfigured = 0,
    Disabled = 1,
    PortBusy = 2,
    NotResponding = 3,
    Online = 4,
}

impl From<i32> for RigStatusX {
    fn from(value: i32) -> Self {
        match value {
            0 => RigStatusX::NotConfigured,
            1 => RigStatusX::Disabled,
            2 => RigStatusX::PortBusy,
            3 => RigStatusX::NotResponding,
            4 => RigStatusX::Online,
            _ => RigStatusX::NotConfigured,
        }
    }
}

impl From<RigStatusX> for i32 {
    fn from(value: RigStatusX) -> Self {
        value as i32
    }
}
