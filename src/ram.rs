use std::vec::Vec;

use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};

use crate::memory::{MemResult, Memory};

/// Basic fixed-size RAM module.
#[derive(Serialize, Deserialize)]
pub struct Ram {
    mem: Vec<u8>,
}

impl Ram {
    /// size in bytes
    pub fn new(size: usize) -> Ram {
        Ram {
            mem: vec![0u8; size],
        }
    }

    pub fn new_with_data(size: usize, data: &[u8]) -> Ram {
        let mut ram = Ram::new(size);
        ram.mem[..data.len()].clone_from_slice(data);
        ram
    }

    pub fn bulk_write(&mut self, offset: usize, data: &[u8]) {
        self.mem[offset..offset + data.len()].copy_from_slice(data)
    }
}

impl Memory for Ram {
    fn label(&self) -> String {
        "Ram".to_string()
    }

    fn r8(&mut self, offset: u32) -> MemResult<u8> {
        let offset = offset as usize;
        Ok(self.mem[offset])
    }

    fn r16(&mut self, offset: u32) -> MemResult<u16> {
        let offset = offset as usize;
        Ok(LittleEndian::read_u16(&self.mem[offset..offset + 2]))
    }

    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        let offset = offset as usize;
        Ok(LittleEndian::read_u32(&self.mem[offset..offset + 4]))
    }

    fn w8(&mut self, offset: u32, val: u8) -> MemResult<()> {
        let offset = offset as usize;
        self.mem[offset] = val;
        Ok(())
    }

    fn w16(&mut self, offset: u32, val: u16) -> MemResult<()> {
        let offset = offset as usize;
        LittleEndian::write_u16(&mut self.mem[offset..offset + 2], val);
        Ok(())
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        let offset = offset as usize;
        LittleEndian::write_u32(&mut self.mem[offset..offset + 4], val);
        Ok(())
    }
}
