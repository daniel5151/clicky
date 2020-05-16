use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// Template for creating new devices.
#[derive(Debug)]
pub struct NewDevice {}

impl NewDevice {
    pub fn new_hle() -> NewDevice {
        NewDevice {}
    }
}

impl Device for NewDevice {
    fn kind(&self) -> &'static str {
        "NewDevice"
    }

    fn probe(&self, offset: u32) -> Probe<'_> {
        let reg = match offset {
            0x0 => "_",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for NewDevice {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0 => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        let _ = val;

        match offset {
            0x0 => return Err(Unimplemented),
            _ => return Err(Unexpected),
        }

        Ok(())
    }
}
