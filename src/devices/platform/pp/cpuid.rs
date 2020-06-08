use crate::devices::prelude::*;

#[derive(Debug)]
pub enum CpuIdKind {
    Cpu,
    Cop,
}

/// Returns different value based on which CPU accesses it.
#[derive(Debug)]
pub struct CpuId {
    cpuid: CpuIdKind,
}

impl CpuId {
    pub fn new() -> CpuId {
        CpuId {
            cpuid: CpuIdKind::Cpu,
        }
    }

    pub fn set_cpuid(&mut self, cpuid: CpuIdKind) {
        self.cpuid = cpuid
    }
}

impl Device for CpuId {
    fn kind(&self) -> &'static str {
        "CPU ID"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x0 => "CPU ID",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for CpuId {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0 => match self.cpuid {
                CpuIdKind::Cpu => Ok(0x55555555),
                CpuIdKind::Cop => Ok(0xaaaaaaaa),
            },
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, _val: u32) -> MemResult<()> {
        match offset {
            0x0 => Err(InvalidAccess),
            _ => Err(Unexpected),
        }
    }
}
