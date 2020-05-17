use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// Poorly documented PP50XX controller
#[derive(Debug)]
pub struct PPCon {
    dev_init: [u32; 2],
}

impl PPCon {
    pub fn new_hle() -> PPCon {
        PPCon { dev_init: [0, 0] }
    }
}

impl Device for PPCon {
    fn kind(&self) -> &'static str {
        "PP Controller (?)"
    }

    fn probe(&self, offset: u32) -> Probe<'_> {
        let reg = match offset {
            0x0 => "ID Reg 1",
            0x4 => "ID Reg 2",
            0x10 => "Dev Init 1",
            0x20 => "Dev Init 2",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for PPCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0 => Ok(u32::from_le_bytes(*b"PP50")),
            0x4 => Ok(u32::from_le_bytes(*b"20D ")),
            0x10 => Err(StubRead(self.dev_init[0])),
            0x20 => Err(StubRead(self.dev_init[1])),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        let _ = val;

        match offset {
            0x0 => Err(InvalidAccess),
            0x4 => Err(InvalidAccess),
            0x10 => {
                self.dev_init[0] = val;
                Err(StubWrite)
            }
            0x20 => {
                self.dev_init[1] = val;
                Err(StubWrite)
            }
            _ => Err(Unexpected),
        }
    }
}
