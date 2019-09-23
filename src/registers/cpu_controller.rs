use crate::memory::{MemResult, Memory};

#[rustfmt::skip]
pub mod flags {
    /* Control flags, can be ORed together */
    pub const FLOW_MASK: u32 = 0b1110 << 28;

    /// Sleep until an interrupt occurs
    pub const PROC_SLEEP: u32    = 0x8000_0000;
    /// Sleep until end of countdown
    pub const PROC_WAIT_CNT: u32 = 0x4000_0000;
    /// Fire interrupt on wake-up. Auto-clears.
    pub const PROC_WAKE_INT: u32 = 0x2000_0000;

    /* Counter source, select one */
    // !! not sure what happens if multiple are set !!

    /// Counter Source - Clock cycles
    pub const PROC_CNT_CLKS: u32 = 0x0800_0000;
    /// Counter Source - Microseconds
    pub const PROC_CNT_USEC: u32 = 0x0200_0000;
    /// Counter Source - Milliseconds
    pub const PROC_CNT_MSEC: u32 = 0x0100_0000;
    /// Counter Source - Seconds. Works on PP5022+ only!
    pub const PROC_CNT_SEC: u32  = 0x0080_0000;
}

// bits 0..7 are the counter (EC on N+1th event)
pub struct CpuController(u32);

impl CpuController {
    pub fn new() -> CpuController {
        CpuController(0)
    }

    pub fn raw(&self) -> u32 {
        self.0
    }

    pub fn get_counter(&self) -> u8 {
        self.0 as u8
    }

    pub fn set_counter(&mut self, val: u8) {
        self.0 = (self.0 & !0xFFu32) | val as u32;
    }
}

impl Memory for CpuController {
    fn label(&self) -> String {
        "CpuController".to_string()
    }

    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        assert!(offset == 0);
        Ok(self.0)
    }
    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        assert!(offset == 0);
        self.0 = val;
        Ok(())
    }
}
