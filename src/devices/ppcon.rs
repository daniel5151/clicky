use log::Level::*;

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// Poorly documented PP50XX controller
#[derive(Debug)]
pub struct PPCon {
    dev_init: [u32; 2],
    gpo_val: u32,
    gpo_enable: u32,
}

impl PPCon {
    pub fn new_hle() -> PPCon {
        PPCon {
            dev_init: [0, 0],
            gpo_enable: 0,
            gpo_val: 0,
        }
    }
}

impl Device for PPCon {
    fn kind(&self) -> &'static str {
        "PP Controller (?)"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x0 => "ID Reg 1",
            0x4 => "ID Reg 2",
            0x10 => "Dev Init 1",
            0x20 => "Dev Init 2",
            0x80 => "GPO32 Val",
            0x84 => "GPO32 Enable",
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
            0x10 => Ok(self.dev_init[0]),
            0x20 => Ok(self.dev_init[1]),
            0x80 => Ok(self.gpo_val),
            0x84 => Ok(self.gpo_enable),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x0 => Err(InvalidAccess),
            0x4 => Err(InvalidAccess),
            0x10 => {
                self.dev_init[0] = val;
                Err(StubWrite(Warn))
            }
            0x20 => {
                self.dev_init[1] = val;
                Err(StubWrite(Warn))
            }
            0x80 => {
                self.gpo_val = val;
                Err(StubWrite(Warn))
            }
            0x84 => {
                self.gpo_enable = val;
                Err(StubWrite(Warn))
            }
            _ => Err(Unexpected),
        }
    }
}
