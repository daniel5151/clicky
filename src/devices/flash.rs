use log::*;

use crate::memory::{MemResult, Memory};
use crate::ram::Ram;

/// Flash ROM (typically dumped from actual hardware)
pub struct Flash {
    // backed by "Ram" which ignores writes
    rom: Ram,
}

impl Flash {
    pub fn new(rom: &[u8]) -> Flash {
        assert!(rom.len() == 0x10_0000);
        Flash {
            rom: Ram::new_with_data(0x10_0000, rom),
        }
    }
}

impl Memory for Flash {
    fn label(&self) -> String {
        "Flash".to_string()
    }

    fn r8(&mut self, offset: u32) -> MemResult<u8> {
        self.rom.r8(offset)
    }

    fn r16(&mut self, offset: u32) -> MemResult<u16> {
        self.rom.r16(offset)
    }

    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        self.rom.r32(offset)
    }

    fn w8(&mut self, offset: u32, val: u8) -> MemResult<()> {
        warn!("w8 {:#04x?} at offset {:#010x?}", val, offset);
        Ok(())
    }

    fn w16(&mut self, offset: u32, val: u16) -> MemResult<()> {
        warn!("w16 {:#06x?} at offset {:#010x?}", val, offset);
        Ok(())
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        warn!("w32 {}:#010x? at offset {:#010x?}", val, offset);
        Ok(())
    }
}
