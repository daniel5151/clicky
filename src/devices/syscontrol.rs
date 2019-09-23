use crate::memory::{MemResult, MemResultExt, Memory};
use crate::registers::{CpuController, CpuId};

use crate::registers::cpu_controller::flags as cpuctl;

pub struct SysControl {
    cpuid: CpuId,
    cpu_controller: CpuController,
    cop_controller: CpuController,
}

impl SysControl {
    pub fn new() -> SysControl {
        SysControl {
            cpuid: CpuId::Cpu,
            cpu_controller: CpuController::new(),
            cop_controller: CpuController::new(),
        }
    }

    pub fn set_cpuid(&mut self, cpuid: CpuId) {
        self.cpuid = cpuid
    }

    pub fn should_cycle_cpu(&self) -> bool {
        // FIXME: this isn't right
        self.cpu_controller.raw() & cpuctl::FLOW_MASK == 0
    }

    pub fn should_cycle_cop(&self) -> bool {
        // FIXME: this isn't right
        self.cop_controller.raw() & cpuctl::FLOW_MASK == 0
    }
}

impl Memory for SysControl {
    fn label(&self) -> String {
        "SysControl".to_string()
    }

    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0000 => self.cpuid.r32(offset),
            0x7000 => self.cpu_controller.r32(offset - 0x7000),
            0x7004 => self.cop_controller.r32(offset - 0x7004),
            _ => crate::unimplemented_offset!(),
        }
        .map_memerr_ctx(offset, self.label())
    }
    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x0000 => self.cpuid.w32(offset, val),
            0x7000 => self.cpu_controller.w32(offset - 0x7000, val),
            0x7004 => self.cop_controller.w32(offset - 0x7004, val),
            _ => crate::unimplemented_offset!(),
        }
        .map_memerr_ctx(offset, self.label())
    }
}
