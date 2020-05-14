use crate::devices::{Device, Interrupt as InterruptMarker, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

#[derive(Debug, Clone, Copy)]
pub enum Interrupt {
    Timer1 = 0,
    Timer2 = 1,
}

impl InterruptMarker for Interrupt {}

#[derive(Debug)]
pub enum CpuId {
    Cpu,
    Cop,
}

#[derive(Debug)]
pub struct SysCon {
    cpuid: CpuId,
    cpuctl: u32,
    copctl: u32,
}

#[allow(dead_code)]
mod cpuctl_flags {
    /* Control flags, can be ORed together */
    pub const FLOW_MASK: u32 = 0b1110 << 28;

    /// Sleep until an interrupt occurs
    pub const PROC_SLEEP: u32 = 0x8000_0000;
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
    pub const PROC_CNT_SEC: u32 = 0x0080_0000;
}

impl SysCon {
    pub fn new_hle() -> SysCon {
        SysCon {
            cpuid: CpuId::Cpu,
            cpuctl: 0x0000_0000,
            copctl: 0x0000_0000,
        }
    }

    pub fn set_cpuid(&mut self, cpuid: CpuId) {
        self.cpuid = cpuid
    }

    pub fn is_cpu_running(&self) -> bool {
        self.cpuctl & cpuctl_flags::FLOW_MASK == 0
    }

    pub fn is_cop_running(&self) -> bool {
        self.copctl & cpuctl_flags::FLOW_MASK == 0
    }

    fn update_cpuctl(&mut self) {
        // TODO: check flow bits and setup timers to wake CPU
    }
}

impl Device for SysCon {
    fn kind(&self) -> &'static str {
        "System Controller Block"
    }

    fn probe(&self, offset: u32) -> Probe<'_> {
        let reg = match offset {
            0x0 => "CPU ID",
            0x7000 => "CPU Control",
            0x7004 => "COP Control",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for SysCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0 => match self.cpuid {
                CpuId::Cpu => Ok(0x55),
                CpuId::Cop => Ok(0xaa),
            },
            0x7000 => Ok(self.cpuctl),
            0x7004 => Ok(self.copctl),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x7000 => {
                log::debug!("updated CPU Control: {:#010x?}", val);
                self.cpuctl = val;
                self.update_cpuctl();
            }
            0x7004 => {
                log::debug!("updated COP Control: {:#010x?}", val);
                self.copctl = val;
                self.update_cpuctl();
            }
            _ => return Err(Unexpected),
        }

        Ok(())
    }
}
