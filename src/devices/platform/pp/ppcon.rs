use crate::devices::prelude::*;

/// Poorly documented PP50XX controller
#[derive(Debug)]
pub struct PPCon {
    dev_init: [u32; 8],
    dev_timing: [u32; 3],
    bootstrap_maybe: [u32; 2],

    gpo_val: u32,
    gpo_enable: u32,
    gpo_input_enable: u32,
}

impl PPCon {
    pub fn new() -> PPCon {
        PPCon {
            dev_init: [0; 8],
            dev_timing: [0; 3],
            bootstrap_maybe: [0; 2],

            gpo_enable: 0,
            gpo_val: 0,
            gpo_input_enable: 0,
        }
    }
}

impl Device for PPCon {
    fn kind(&self) -> &'static str {
        "PP Controller"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "ID Reg 1",
            0x04 => "ID Reg 2",
            0x08 => "(?) Bootstrap 1",
            0x0c => "(?) Bootstrap 2",
            0x10 => "Dev Init 1",
            0x14 => "(?) Dev Init 1.1",
            0x18 => "(?) Dev Init 1.2",
            0x1c => "(?) Dev Init 1.3",
            0x20 => "Dev Init 2",
            0x24 => "(?) Dev Init 2.1",
            0x28 => "(?) Dev Init 2.2 (USB related)",
            0x2c => "(?) Dev Init 2.3 (USB related)",
            0x30 => "(?) Dev Timing 0",
            0x34 => "Dev Timing 1",
            0x3c => "(?) Dev Timing 1.1",
            0x80 => "GPO32 Val",
            0x84 => "GPO32 Enable",
            0x88 => "GPO32 Input",
            0x8c => "GPO32 Input Enable",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for PPCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => Ok(u32::from_le_bytes(*b"PP50")),
            0x04 => Ok(u32::from_le_bytes(*b"20D ")),
            0x08 => Err(StubRead(Info, self.bootstrap_maybe[0])),
            0x0c => Err(StubRead(Info, self.bootstrap_maybe[1])),
            0x10 => Err(StubRead(Info, self.dev_init[0])),
            0x14 => Err(StubRead(Info, self.dev_init[1])),
            0x18 => Err(StubRead(Info, self.dev_init[2])),
            0x1c => Err(StubRead(Info, self.dev_init[3])),
            0x20 => Err(StubRead(Info, self.dev_init[4])),
            0x24 => Err(StubRead(Info, self.dev_init[5])),
            // HACK: flag needs to be set to progress through USB init in rockbox
            0x28 => Err(StubRead(Info, self.dev_init[6] | 0x80)),
            0x2c => Err(StubRead(Info, self.dev_init[7])),
            0x30 => Err(StubRead(Info, self.dev_timing[0])),
            0x34 => Err(StubRead(Info, self.dev_timing[1])),
            0x3c => Err(StubRead(Info, self.dev_timing[2])),
            0x80 => Ok(self.gpo_val),
            0x84 => Ok(self.gpo_enable),
            0x88 => Err(StubRead(Info, 0x00)),
            0x8c => Ok(self.gpo_input_enable),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => Err(InvalidAccess),
            0x04 => Err(InvalidAccess),
            0x08 => Err(StubWrite(Info, self.bootstrap_maybe[0] = val)),
            0x0c => Err(StubWrite(Info, self.bootstrap_maybe[1] = val)),
            0x10 => Err(StubWrite(Info, self.dev_init[0] = val)),
            0x14 => Err(StubWrite(Info, self.dev_init[1] = val)),
            0x18 => Err(StubWrite(Info, self.dev_init[2] = val)),
            0x1c => Err(StubWrite(Info, self.dev_init[3] = val)),
            0x20 => Err(StubWrite(Info, self.dev_init[4] = val)),
            0x24 => Err(StubWrite(Info, self.dev_init[5] = val)),
            0x28 => Err(StubWrite(Info, self.dev_init[6] = val)),
            0x2c => Err(StubWrite(Info, self.dev_init[7] = val)),
            // HACK: flag needs to be set to progress through the Flash ROM bootloader
            0x30 => Err(StubWrite(Info, self.dev_timing[0] = val | 0x8000000)),
            0x34 => Err(StubWrite(Info, self.dev_timing[1] = val)),
            // HACK: flag needs to be set to progress through the Flash ROM bootloader
            0x3c => Err(StubWrite(Info, self.dev_timing[2] = val | 0x80000000)),
            0x80 => Ok(self.gpo_val = val),
            0x84 => Ok(self.gpo_enable = val),
            0x88 => Err(InvalidAccess),
            0x8c => Ok(self.gpo_input_enable = val),
            _ => Err(Unexpected),
        }
    }
}
