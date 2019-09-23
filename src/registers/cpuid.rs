use crate::memory::{MemResult, Memory};

/// Returns a constant value based off which CPU is performing the access
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum CpuId {
    Cpu = 0x55,
    Cop = 0xaa,
}

impl Memory for CpuId {
    fn label(&self) -> String {
        "CpuId".to_string()
    }

    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        assert!(offset == 0);
        Ok(*self as u32)
    }
    fn w32(&mut self, offset: u32, _val: u32) -> MemResult<()> {
        assert!(offset == 0);
        Ok(())
        // do nothing
    }
}
