use crate::devices::prelude::*;

use std::vec::Vec;

use byteorder::{ByteOrder, LittleEndian};

/// Basic RAM device.
pub struct Ram {
    mem: Vec<u8>,
}

impl std::fmt::Debug for Ram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ram").field("mem", &"[..]").finish()
    }
}

impl Ram {
    /// Allocate some RAM. `size` is the size in bytes.
    pub fn new(size: usize) -> Ram {
        Ram {
            mem: vec![b'-'; size], // non-zero value to make it easier to spot bugs
        }
    }

    pub fn bulk_write(&mut self, offset: u32, data: &[u8]) {
        let offset = offset as usize;
        self.mem[offset..offset + data.len()].copy_from_slice(data);
    }

    pub fn bulk_read(&self, offset: u32, data: &mut [u8]) {
        let offset = offset as usize;
        data.copy_from_slice(&self.mem[offset..offset + data.len()]);
    }
}

impl Device for Ram {
    fn kind(&self) -> &'static str {
        "Ram"
    }

    fn probe(&self, offset: u32) -> Probe {
        if (offset as usize) < self.mem.len() {
            Probe::Register("<data>")
        } else {
            Probe::Unmapped
        }
    }
}

impl Memory for Ram {
    #[inline]
    fn r8(&mut self, offset: u32) -> MemResult<u8> {
        let offset = offset as usize;
        let val = self.mem[offset];
        Ok(val)
    }

    #[inline]
    fn r16(&mut self, offset: u32) -> MemResult<u16> {
        let offset = offset as usize;
        let val = LittleEndian::read_u16(&self.mem[offset..]);
        Ok(val)
    }

    #[inline]
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        let offset = offset as usize;
        let val = LittleEndian::read_u32(&self.mem[offset..]);
        Ok(val)
    }

    #[inline]
    fn w8(&mut self, offset: u32, val: u8) -> MemResult<()> {
        let offset = offset as usize;
        self.mem[offset] = val;
        Ok(())
    }

    #[inline]
    fn w16(&mut self, offset: u32, val: u16) -> MemResult<()> {
        let offset = offset as usize;
        LittleEndian::write_u16(&mut self.mem[offset..], val);
        Ok(())
    }

    #[inline]
    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        let offset = offset as usize;
        LittleEndian::write_u32(&mut self.mem[offset..], val);
        Ok(())
    }
}
