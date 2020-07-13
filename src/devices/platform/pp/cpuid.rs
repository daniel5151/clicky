use crate::devices::prelude::*;

use super::common::CpuId;

/// Returns different value based on which CPU accesses it.
#[derive(Debug)]
pub struct CpuIdReg {
    cpuid: CpuId,
}

impl CpuIdReg {
    pub fn new() -> CpuIdReg {
        CpuIdReg { cpuid: CpuId::Cpu }
    }

    pub fn set_cpuid(&mut self, cpuid: CpuId) {
        self.cpuid = cpuid
    }
}

impl Device for CpuIdReg {
    fn kind(&self) -> &'static str {
        "CPU ID Register"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x0 => "CPU ID Register",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for CpuIdReg {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0 => match self.cpuid {
                CpuId::Cpu => Ok(0x55555555),
                CpuId::Cop => Ok(0xaaaaaaaa),
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
