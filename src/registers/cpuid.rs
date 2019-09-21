use crate::memory::WordAlignedMemory;

/// Returns a constant value based off which CPU is performing the access
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum CpuId {
    Cpu = 0x55,
    Cop = 0xaa,
}

impl WordAlignedMemory for CpuId {
    fn r32(&mut self, offset: u32) -> u32 {
        assert!(offset == 0);
        *self as u32
    }
    fn w32(&mut self, offset: u32, _val: u32) {
        assert!(offset == 0);
        // do nothing
    }
}
