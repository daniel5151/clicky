use log::Level::*;

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// PP5020 I2S controller.
#[derive(Debug)]
pub struct I2SCon {
    config: u32,
    clock: u32,
    fifo_cfg: u32,
}

impl I2SCon {
    pub fn new() -> I2SCon {
        I2SCon {
            config: 0,
            clock: 0,
            fifo_cfg: 0,
        }
    }
}

impl Device for I2SCon {
    fn kind(&self) -> &'static str {
        "I2S Controller"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "Config",
            0x08 => "Clock",
            0x0c => "Fifo Config",
            0x40 => "Fifo Write",
            0x80 => "Fifo Read",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for I2SCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => Err(StubRead(Error, self.config)),
            0x08 => Err(StubRead(Error, self.clock)),
            0x0c => Err(StubRead(Error, self.fifo_cfg)),
            0x40 => Err(Unimplemented),
            0x80 => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => Err(StubWrite(Error, self.config = val)),
            0x08 => Err(StubWrite(Error, self.clock = val)),
            0x0c => Err(StubWrite(Error, self.fifo_cfg = val)),
            0x40 => Err(Unimplemented),
            0x80 => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }
}
