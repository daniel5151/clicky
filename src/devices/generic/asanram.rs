use crate::devices::prelude::*;

use std::vec::Vec;

use byteorder::{ByteOrder, LittleEndian};

/// RAM device which raises ContractViolation warnings when reading from
/// uninitialized memory.
pub struct AsanRam {
    mem: Vec<u8>,
    initialized: Vec<bool>,
    only_warn_once: bool,
}

impl std::fmt::Debug for AsanRam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsanRam")
            .field("mem", &"<omitted>")
            .field("initialized", &"<omitted>")
            .finish()
    }
}

impl AsanRam {
    /// Create a new AsanRAM.
    ///
    /// - `size` is the size in bytes.
    /// - Setting `only_warn_once` will only emit a warning the _first_ time
    ///   uninitialized memory is read.
    pub fn new(size: usize, only_warn_once: bool) -> AsanRam {
        AsanRam {
            mem: vec![b'-'; size], // non-zero value to make it easier to spot bugs
            initialized: vec![false; size],
            only_warn_once,
        }
    }

    pub fn bulk_write(&mut self, offset: u32, data: &[u8]) {
        let offset = offset as usize;
        self.mem[offset..offset + data.len()].copy_from_slice(data);
        self.initialized[offset..offset + data.len()]
            .iter_mut()
            .for_each(|b| *b = true);
    }

    fn uninit_read(&self, offset: usize, size: usize, stub: u32) -> MemException {
        let mut partially_init = false;
        let data = self.initialized[offset..offset + size]
            .iter()
            .zip(self.mem[offset..offset + size].iter())
            .map(|(init, val)| {
                if *init {
                    partially_init = true;
                    format!("{:02x?}", val)
                } else {
                    "??".to_string()
                }
            })
            .collect::<String>();

        let msg = if partially_init {
            format!("r{} from partially uninitialized RAM: 0x{}", size * 8, data)
        } else {
            format!("r{} from uninitialized RAM", size * 8,)
        };

        ContractViolation {
            msg,
            severity: log::Level::Warn,
            stub_val: Some(stub as u32),
        }
    }
}

impl Device for AsanRam {
    fn kind(&self) -> &'static str {
        "AsanRam"
    }

    fn probe(&self, offset: u32) -> Probe {
        assert!((offset as usize) < self.mem.len());

        Probe::Register("<data>")
    }
}

impl Memory for AsanRam {
    fn r8(&mut self, offset: u32) -> MemResult<u8> {
        let offset = offset as usize;
        let val = self.mem[offset];
        if !self.initialized[offset] {
            let err = Err(self.uninit_read(offset, 1, val as u32));
            if self.only_warn_once {
                self.initialized[offset] = true;
            }
            return err;
        }
        Ok(val)
    }

    fn r16(&mut self, offset: u32) -> MemResult<u16> {
        let offset = offset as usize;
        let val = LittleEndian::read_u16(&self.mem[offset..offset + 2]);
        if self.initialized[offset..offset + 2] != [true; 2] {
            let err = Err(self.uninit_read(offset, 2, val as u32));
            if self.only_warn_once {
                self.initialized[offset..offset + 2].copy_from_slice(&[true; 2]);
            }
            return err;
        }
        Ok(val)
    }

    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        let offset = offset as usize;
        let val = LittleEndian::read_u32(&self.mem[offset..offset + 4]);
        if self.initialized[offset..offset + 4] != [true; 4] {
            // gcc likes to emit 8-bit store instructions, but later read those values via
            // 32 bit read instructions. To squelch these errors, word-aligned reads are
            // allowed to return partially uninitialized words.
            if self.initialized[offset & !0x3] {
                return Ok(val);
            } else {
                let err = Err(self.uninit_read(offset, 4, val));
                if self.only_warn_once {
                    self.initialized[offset..offset + 4].copy_from_slice(&[true; 4]);
                }
                return err;
            }
        }
        Ok(val)
    }

    fn w8(&mut self, offset: u32, val: u8) -> MemResult<()> {
        let offset = offset as usize;
        self.initialized[offset] = true;
        self.mem[offset] = val;
        Ok(())
    }

    fn w16(&mut self, offset: u32, val: u16) -> MemResult<()> {
        let offset = offset as usize;
        self.initialized[offset..offset + 2].copy_from_slice(&[true; 2]);
        LittleEndian::write_u16(&mut self.mem[offset..offset + 2], val);
        Ok(())
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        let offset = offset as usize;
        self.initialized[offset..offset + 4].copy_from_slice(&[true; 4]);
        LittleEndian::write_u32(&mut self.mem[offset..offset + 4], val);
        Ok(())
    }
}
