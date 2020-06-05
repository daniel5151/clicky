use log::Level::*;

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// I2C Controller
#[derive(Debug)]
pub struct I2CCon {
    control: u32,
}

impl I2CCon {
    pub fn new() -> I2CCon {
        I2CCon { control: 0 }
    }
}

impl Device for I2CCon {
    fn kind(&self) -> &'static str {
        "I2CCon"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x000 => "Control",
            0x004 => "Addr",
            0x00c => "Data0",
            0x010 => "Data1",
            0x014 => "Data2",
            0x018 => "Data3",
            0x01c => "Status",
            0x100 => "?",
            0x104 => "?",
            0x120 => "?",
            0x124 => "?",
            0x140 => "Scroll Wheel + Keypad Buttons",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for I2CCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x000 => Err(StubRead(Warn, self.control)),
            0x004 => Err(StubRead(Warn, 0)),
            0x00c => Err(StubRead(Warn, 0)),
            0x010 => Err(StubRead(Warn, 0)),
            0x014 => Err(StubRead(Warn, 0)),
            0x018 => Err(StubRead(Warn, 0)),
            0x01c => Err(StubRead(Warn, 0)),
            0x100 => Err(StubRead(Warn, 0)),
            0x104 => Err(StubRead(Warn, 0)),
            0x120 => Err(StubRead(Warn, 0)),
            0x124 => Err(StubRead(Warn, 0)),
            0x140 => Err(StubRead(Warn, 0)),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x000 => Err(StubWrite(Warn, self.control = val)),
            0x004 => Err(StubWrite(Warn, ())),
            0x00c => Err(StubWrite(Warn, ())),
            0x010 => Err(StubWrite(Warn, ())),
            0x014 => Err(StubWrite(Warn, ())),
            0x018 => Err(StubWrite(Warn, ())),
            0x01c => Err(StubWrite(Warn, ())),
            0x100 => Err(StubWrite(Warn, ())),
            0x104 => Err(StubWrite(Warn, ())),
            0x120 => Err(StubWrite(Warn, ())),
            0x124 => Err(StubWrite(Warn, ())),
            0x140 => Err(StubWrite(Warn, ())),
            _ => Err(Unexpected),
        }
    }
}
