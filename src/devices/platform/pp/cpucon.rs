use crate::devices::prelude::*;

pub use super::common::CpuId;

/// PP5020 CPU controller
#[derive(Debug)]
pub struct CpuCon {
    cpuctl: u32,
    copctl: u32,
}

#[allow(dead_code)]
mod flags {
    type Range = std::ops::RangeInclusive<usize>;

    /* Control flags, can be ORed together */
    pub const FLOW_MASK: Range = 29..=31;

    /// Sleep until an interrupt occurs
    pub const PROC_SLEEP: usize = 31;
    /// Sleep until end of countdown
    pub const PROC_WAIT_CNT: usize = 30;
    /// Fire interrupt on wake-up. Auto-clears.
    pub const PROC_WAKE_INT: usize = 29;

    /* Counter source, select one */
    // !! not sure what happens if multiple are set !!

    /// Counter Source - Clock cycles
    pub const PROC_CNT_CLKS: usize = 27;
    /// Counter Source - Microseconds
    pub const PROC_CNT_USEC: usize = 25;
    /// Counter Source - Milliseconds
    pub const PROC_CNT_MSEC: usize = 24;
    /// Counter Source - Seconds. Works on PP5022+ only!
    pub const PROC_CNT_SEC: usize = 23;

    pub const COUNTER: Range = 0..=7;
}

impl CpuCon {
    pub fn new() -> CpuCon {
        CpuCon {
            cpuctl: 0x0000_0000,
            copctl: 0x0000_0000,
        }
    }

    pub fn is_cpu_running(&self) -> bool {
        // this ain't it chief
        self.cpuctl.get_bits(flags::FLOW_MASK) == 0
    }

    pub fn is_cop_running(&self) -> bool {
        // this ain't it chief
        self.copctl.get_bits(flags::FLOW_MASK) == 0
    }

    fn update_cpuctl(&mut self, cpu: CpuId) -> MemResult<()> {
        let reg = match cpu {
            CpuId::Cpu => &mut self.cpuctl,
            CpuId::Cop => &mut self.copctl,
        };

        if reg.get_bit(flags::PROC_CNT_CLKS)
            || reg.get_bit(flags::PROC_CNT_USEC)
            || reg.get_bit(flags::PROC_CNT_MSEC)
            || reg.get_bit(flags::PROC_CNT_SEC)
        {
            return Err(Fatal(format!(
                "unimplemented: tried to set cpuctl counter for {:?}",
                cpu
            )));
        }

        if reg.get_bit(flags::PROC_WAIT_CNT) {
            return Err(Fatal(format!(
                "unimplemented: 'Sleep until end of countdown' for {:?}",
                cpu
            )));
        }

        if reg.get_bit(flags::PROC_WAKE_INT) {
            return Err(Fatal(format!(
                "unimplemented: 'Fire interrupt on wake-up' for {:?}",
                cpu
            )));
        }

        // TODO: check flow bits and setup timers to wake CPU
        Ok(())
    }
}

impl Device for CpuCon {
    fn kind(&self) -> &'static str {
        "System Controller Block"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x0 => "CPU Control",
            0x4 => "COP Control",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for CpuCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0 => Ok(self.cpuctl),
            0x4 => Ok(self.copctl),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x0 => {
                self.cpuctl = val;
                self.update_cpuctl(CpuId::Cpu)?;
                Err(Log(Debug, format!("updated CPU Control: {:#010x?}", val)))
            }
            0x4 => {
                self.copctl = val;
                self.update_cpuctl(CpuId::Cop)?;
                Err(Log(Debug, format!("updated COP Control: {:#010x?}", val)))
            }
            _ => Err(Unexpected),
        }
    }
}
