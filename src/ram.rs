use std::vec::Vec;

use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};

use arm7tdmi_rs::Memory;

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
    fn r8(&mut self, addr: u32) -> u8 {
        let idx = addr as usize;
        if idx < self.mem.len() {
            self.mem[idx]
        } else {
            panic!("p8 from invalid address {:#010x}", addr);
        }
    }

    fn r16(&mut self, addr: u32) -> u16 {
        debug_assert!(addr % 2 == 0);

        let idx = addr as usize;
        if idx < self.mem.len() - 1 {
            LittleEndian::read_u16(&self.mem[idx..idx + 2])
        } else {
            panic!("p16 from invalid address {:#010x}", addr);
        }
    }

    fn r32(&mut self, addr: u32) -> u32 {
        debug_assert!(addr % 4 == 0);

        let idx = addr as usize;
        if idx < self.mem.len() - 3 {
            LittleEndian::read_u32(&self.mem[idx..idx + 4])
        } else {
            panic!("p32 from invalid address {:#010x}", addr);
        }
    }

    fn w8(&mut self, addr: u32, val: u8) {
        let idx = addr as usize;
        if idx < self.mem.len() {
            self.mem[idx] = val;
        } else {
            panic!("w8 to invalid address {:#010x}", addr);
        }
    }

    fn w16(&mut self, addr: u32, val: u16) {
        debug_assert!(addr % 2 == 0);

        let idx = addr as usize;
        if idx < self.mem.len() - 1 {
            LittleEndian::write_u16(&mut self.mem[idx..idx + 2], val);
        } else {
            panic!("w16 to invalid address {:#010x}", addr);
        }
    }

    fn w32(&mut self, addr: u32, val: u32) {
        debug_assert!(addr % 4 == 0);

        let idx = addr as usize;
        if idx < self.mem.len() - 3 {
            LittleEndian::write_u32(&mut self.mem[idx..idx + 4], val);
        } else {
            panic!("w32 to invalid address {:#010x}", addr);
        }
    }
}
