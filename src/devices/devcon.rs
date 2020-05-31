use log::Level::*;

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// PP5020 Device Controller.
#[derive(Debug)]
pub struct DevCon {
    reset: [u32; 2],
    enable: [u32; 2],
    mystery_i2c: u8,
}

impl DevCon {
    pub fn new_hle() -> DevCon {
        DevCon {
            reset: [0, 0],
            enable: [0, 0],
            mystery_i2c: 0,
        }
    }
}

impl Device for DevCon {
    fn kind(&self) -> &'static str {
        "DevCon"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x04 => "Device Reset 1",
            0x08 => "Device Reset 2",
            0x0c => "Device Enable 1",
            0x10 => "Device Enable 2",
            0xa4 => "Mystery I2C reg (?)",
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
            0xa4 => Err(StubRead(Error, self.mystery_i2c as u32)),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x04 => Err(StubWrite(Warn, self.reset[0] = val)),
            0x08 => Err(StubWrite(Warn, self.reset[1] = val)),
            0x0c => Err(StubWrite(Info, self.enable[0] = val)),
            0x10 => Err(StubWrite(Info, self.enable[1] = val)),
            0xa4 => Err(StubWrite(Error, self.mystery_i2c = val as u8)),
            _ => Err(Unexpected),
        }
    }
}
