use log::Level::*;

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// PP5020 Device Controller.
#[derive(Debug)]
pub struct DevCon {
    reset: [u32; 2],
    enable: [u32; 2],
}

impl DevCon {
    pub fn new_hle() -> DevCon {
        DevCon {
            reset: [0, 0],
            enable: [0, 0],
        }
    }
}

impl Device for DevCon {
    fn kind(&self) -> &'static str {
        "DevCon"
    }

    fn probe(&self, offset: u32) -> Probe<'_> {
        let reg = match offset {
            0x04 => "Device Reset 1",
            0x08 => "Device Reset 2",
            0x0c => "Device Enable 1",
            0x10 => "Device Enable 2",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for DevCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x04 => Ok(self.reset[0]),
            0x08 => Ok(self.reset[1]),
            0x0c => Ok(self.enable[0]),
            0x10 => Ok(self.enable[1]),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x04 => {
                self.reset[0] = val;
                Err(StubWrite(Warn))
            }
            0x08 => {
                self.reset[1] = val;
                Err(StubWrite(Warn))
            }
            0x0c => {
                self.enable[0] = val;
                Err(StubWrite(Warn))
            }
            0x10 => {
                self.enable[1] = val;
                Err(StubWrite(Warn))
            }
            _ => Err(Unexpected),
        }
    }
}
