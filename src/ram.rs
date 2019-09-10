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
}

impl Memory for Ram {
    // reads are side-effect free
    fn r8(&mut self, addr: u32) -> u8 {
        self.p8(addr)
    }

    fn r16(&mut self, addr: u32) -> u16 {
        self.p16(addr)
    }

    fn r32(&mut self, addr: u32) -> u32 {
        self.p32(addr)
    }

    fn p8(&self, addr: u32) -> u8 {
        let idx = addr as usize;
        if idx < self.mem.len() {
            self.mem[idx]
        } else {
            panic!("p8 from invalid address {:#08x}", addr);
        }
    }

    fn p16(&self, addr: u32) -> u16 {
        debug_assert!(addr % 2 == 0);

        let idx = addr as usize;
        if idx < self.mem.len() - 1 {
            LittleEndian::read_u16(&self.mem[idx..idx + 2])
        } else {
            panic!("p16 from invalid address {:#08x}", addr);
        }
    }

    fn p32(&self, addr: u32) -> u32 {
        debug_assert!(addr % 4 == 0);

        let idx = addr as usize;
        if idx < self.mem.len() - 3 {
            LittleEndian::read_u32(&self.mem[idx..idx + 4])
        } else {
            panic!("p32 from invalid address {:#08x}", addr);
        }
    }

    fn w8(&mut self, addr: u32, val: u8) {
        let idx = addr as usize;
        if idx < self.mem.len() {
            self.mem[idx] = val;
        } else {
            panic!("w8 to invalid address {:#08x}", addr);
        }
    }

    fn w16(&mut self, addr: u32, val: u16) {
        debug_assert!(addr % 2 == 0);

        let idx = addr as usize;
        if idx < self.mem.len() - 1 {
            LittleEndian::write_u16(&mut self.mem[idx..idx + 2], val);
        } else {
            panic!("w16 to invalid address {:#08x}", addr);
        }
    }

    fn w32(&mut self, addr: u32, val: u32) {
        debug_assert!(addr % 4 == 0);

        let idx = addr as usize;
        if idx < self.mem.len() - 3 {
            LittleEndian::write_u32(&mut self.mem[idx..idx + 4], val);
        } else {
            panic!("w32 to invalid address {:#08x}", addr);
        }
    }
}
