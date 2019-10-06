use log::info;

use crate::memory::{MemResult, Memory};

/// A transparent wrapper around memory objects that logs any reads / writes
pub struct MemLogger<M: Memory>(M);

impl<M: Memory> MemLogger<M> {
    pub fn new(memory: M) -> MemLogger<M> {
        MemLogger(memory)
    }
}

/// if the given bytes correspond to something vaguely resembling an
/// ascii-string, return a non-empty string with those chars
fn fmt_bstr(bstr: &[u8]) -> String {
    let s = String::from_utf8(
        bstr.iter()
            .map(|b| std::ascii::escape_default(*b))
            .flatten()
            .collect::<Vec<u8>>(),
    )
    .unwrap();

    if s.len() == bstr.len() * 4 {
        String::new()
    } else {
        format!("# b\"{}\"", s)
    }
}

impl<M: Memory> Memory for MemLogger<M> {
    fn label(&self) -> String {
        self.0.label()
    }

    fn r8(&mut self, offset: u32) -> MemResult<u8> {
        let res = self.0.r8(offset)?;
        info!(
            "[{}] r8({:#010x?}) -> 0x{:02x} {}",
            self.label(),
            offset,
            res,
            fmt_bstr(&res.to_le_bytes())
        );
        Ok(res)
    }
    fn r16(&mut self, offset: u32) -> MemResult<u16> {
        let res = self.0.r16(offset)?;
        info!(
            "[{}] r16({:#010x?}) -> 0x{:04x} {}",
            self.label(),
            offset,
            res,
            fmt_bstr(&res.to_le_bytes())
        );
        Ok(res)
    }
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        let res = self.0.r32(offset)?;
        info!(
            "[{}] r32({:#010x?}) -> 0x{:08x} {}",
            self.label(),
            offset,
            res,
            fmt_bstr(&res.to_le_bytes())
        );
        Ok(res)
    }

    fn w8(&mut self, offset: u32, val: u8) -> MemResult<()> {
        self.0.w8(offset, val)?;
        info!("[{}] w8({:#010x?}, {:#04x?})", self.label(), offset, val);
        Ok(())
    }
    fn w16(&mut self, offset: u32, val: u16) -> MemResult<()> {
        self.0.w16(offset, val)?;
        info!("[{}] w16({:#010x?}, {:#06x?})", self.label(), offset, val);
        Ok(())
    }
    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        self.0.w32(offset, val)?;
        info!("[{}] w32({:#010x?}, {:#010x?})", self.label(), offset, val);
        Ok(())
    }
}
