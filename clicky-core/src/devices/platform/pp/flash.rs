use crate::devices::prelude::*;

use byteorder::{ByteOrder, LittleEndian};

/// Internal iPod Flash ROM. Defaults to HLE mode (where only a few critical
/// memory locations can be read). Use the `use_dump` method if you have a dump
/// of a real iPod's flash ROM.
pub struct Flash {
    dump: Option<Box<[u8]>>,
}

impl std::fmt::Debug for Flash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Flash")
            .field("dump", &self.dump.as_ref().map(|_| "[...]"))
            .finish()
    }
}

impl Flash {
    pub fn new() -> Flash {
        Flash { dump: None }
    }

    pub fn use_dump(&mut self, dump: Box<[u8]>) -> Result<(), &'static str> {
        if dump.len() != 0x100000 {
            return Err("Flash ROM dump must be exactly 1MB");
        }
        self.dump = Some(dump);
        Ok(())
    }

    pub fn is_hle(&self) -> bool {
        self.dump.is_none()
    }

    fn hle_vals(offset: u32) -> MemResult<u32> {
        match offset {
            // idk what ipodloader/tools.c:get_ipod_rev() is doing lol
            0x2000 => Ok(u32::from_le_bytes(*b"gfCS")),
            // hardware revision magic number
            // see: https://www.rockbox.org/wiki/IpodHardwareInfo
            0x2084 => Ok(0x0005_0014), // iPod 4th Gen
            _ => Err(Unimplemented),
        }
    }
}

impl Device for Flash {
    fn kind(&self) -> &'static str {
        "Flash Rom"
    }

    fn label(&self) -> Option<&'static str> {
        Some(if self.is_hle() { "HLE" } else { "Dumped" })
    }

    fn probe(&self, offset: u32) -> Probe {
        if offset > 0xFFFFF {
            Probe::Unmapped
        } else {
            Probe::Register("<flash rom>")
        }
    }
}

impl Memory for Flash {
    fn r8(&mut self, offset: u32) -> MemResult<u8> {
        if offset > 0xFFFFF {
            return Err(Unexpected);
        }

        if let Some(dump) = self.dump.as_ref() {
            let offset = offset as usize;
            let val = dump[offset];
            return Ok(val);
        }

        // don't support unaligned HLE reads
        Err(Unimplemented)
    }

    fn r16(&mut self, offset: u32) -> MemResult<u16> {
        if offset > 0xFFFFF {
            return Err(Unexpected);
        }

        if let Some(dump) = self.dump.as_ref() {
            let offset = offset as usize;
            let val = LittleEndian::read_u16(&dump[offset..offset + 2]);
            return Ok(val);
        }

        // don't support unaligned HLE reads
        Err(Unimplemented)
    }

    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        if offset > 0xFFFFF {
            return Err(Unexpected);
        }

        if let Some(dump) = self.dump.as_ref() {
            let offset = offset as usize;
            let val = LittleEndian::read_u32(&dump[offset..offset + 4]);
            return Ok(val);
        }

        Self::hle_vals(offset)
    }

    fn w8(&mut self, _offset: u32, _val: u8) -> MemResult<()> {
        Err(StubWrite(Warn, ()))
    }

    fn w16(&mut self, _offset: u32, _val: u16) -> MemResult<()> {
        Err(StubWrite(Warn, ()))
    }

    fn w32(&mut self, _offset: u32, _val: u32) -> MemResult<()> {
        Err(StubWrite(Warn, ()))
    }
}
