use log::Level::*;

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// PP5020 Interrupt Controller
#[derive(Debug, Default)]
pub struct IntCon {
    label: &'static str,

    cpu_int_stat: u32,
    cop_int_stat: u32,
    cpu_fiq_stat: u32,
    cop_fiq_stat: u32,

    int_stat: u32,
    int_forced_stat: u32,
    int_forced_set: u32,
    int_forced_clr: u32,

    cpu_int_enable_stat: u32,
    cpu_int_enable: u32,
    cpu_int_disable: u32,
    cpu_int_priority: u32,

    cop_int_enable_stat: u32,
    cop_int_enable: u32,
    cop_int_disable: u32,
    cop_int_priority: u32,
}

impl IntCon {
    pub fn new_hle(label: &'static str) -> IntCon {
        IntCon {
            label,

            cpu_int_stat: 0,
            cop_int_stat: 0,
            cpu_fiq_stat: 0,
            cop_fiq_stat: 0,

            int_stat: 0,
            int_forced_stat: 0,
            int_forced_set: 0,
            int_forced_clr: 0,

            cpu_int_enable_stat: 0,
            cpu_int_enable: 0,
            cpu_int_disable: 0,
            cpu_int_priority: 0,

            cop_int_enable_stat: 0,
            cop_int_enable: 0,
            cop_int_disable: 0,
            cop_int_priority: 0,
        }
    }
}

impl Device for IntCon {
    fn kind(&self) -> &'static str {
        "Interrupt Controller"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "CpuIntStat",
            0x04 => "CopIntStat",
            0x08 => "CpuFiqStat",
            0x0c => "CopFiqStat",
            0x10 => "IntStat",
            0x14 => "IntForcedStat",
            0x18 => "IntForcedSet",
            0x1c => "IntForcedClr",
            0x20 => "CpuIntEnableStat",
            0x24 => "CpuIntEnable",
            0x28 => "CpuIntDisable",
            0x2c => "CpuIntPriority",
            0x30 => "CopIntEnableStat",
            0x34 => "CopIntEnable",
            0x38 => "CopIntDisable",
            0x3c => "CopIntPriority",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for IntCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => Err(Unimplemented),
            0x04 => Err(Unimplemented),
            0x08 => Err(Unimplemented),
            0x0c => Err(Unimplemented),
            0x10 => Err(Unimplemented),
            0x14 => Err(Unimplemented),
            0x18 => Err(Unimplemented),
            0x1c => Err(Unimplemented),
            0x20 => Err(Unimplemented),
            0x24 => Err(Unimplemented),
            0x28 => Err(Unimplemented),
            0x2c => Err(Unimplemented),
            0x30 => Err(Unimplemented),
            0x34 => Err(Unimplemented),
            0x38 => Err(Unimplemented),
            0x3c => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => Err(Unimplemented),
            0x04 => Err(Unimplemented),
            0x08 => Err(Unimplemented),
            0x0c => Err(Unimplemented),
            0x10 => Err(Unimplemented),
            0x14 => Err(Unimplemented),
            0x18 => Err(Unimplemented),
            0x1c => Err(Unimplemented),
            0x20 => Err(Unimplemented),
            0x24 => {
                self.cpu_int_enable = val;
                Err(StubWrite(Warn))
            }
            0x28 => Err(Unimplemented),
            0x2c => Err(Unimplemented),
            0x30 => Err(Unimplemented),
            0x34 => Err(Unimplemented),
            0x38 => Err(Unimplemented),
            0x3c => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }
}
