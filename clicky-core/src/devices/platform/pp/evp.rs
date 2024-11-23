use crate::devices::prelude::*;

/// Returns different value based on which CPU accesses it.
#[derive(Debug)]
pub struct Evp {
    reset_vec: u32,
    undefined_instr_vec: u32,
    soft_irq_vec: u32,
    prefetch_abrt_vec: u32,
    data_abrt_vec: u32,
    reserved_vec: u32,
    normal_irq_vec: u32,
    high_priority_irq_vec: u32,
}

impl Evp {
    pub fn new() -> Evp {
        Evp {
            reset_vec: 0x0,
            undefined_instr_vec: 0x4,
            soft_irq_vec: 0x8,
            prefetch_abrt_vec: 0xC,
            data_abrt_vec: 0x10,
            reserved_vec: 0x14,
            normal_irq_vec: 0x18,
            high_priority_irq_vec: 0x1C,
        }
    }
}

impl Device for Evp {
    fn kind(&self) -> &'static str {
        "EVP (Exception Vector ???)"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x0 => "Reset Exception Handler",
            0x4 => "Undefined Instruction Handler",
            0x8 => "Software Interrupt Handler",
            0xC => "Prefetch Abort Handler",
            0x10 => "Data Abort Handler",
            0x14 => "Reserved Handler",
            0x18 => "Normal-priority Interrupt Handler",
            0x1C => "High-priority Interrupt Handler",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for Evp {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0 => Ok(self.reset_vec),
            0x4 => Ok(self.undefined_instr_vec),
            0x8 => Ok(self.soft_irq_vec),
            0xC => Ok(self.prefetch_abrt_vec),
            0x10 => Ok(self.data_abrt_vec),
            0x14 => Ok(self.reserved_vec),
            0x18 => Ok(self.normal_irq_vec),
            0x1C => Ok(self.high_priority_irq_vec),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x0 => Ok(self.reset_vec = val),
            0x4 => Ok(self.undefined_instr_vec = val),
            0x8 => Ok(self.soft_irq_vec = val),
            0xC => Ok(self.prefetch_abrt_vec = val),
            0x10 => Ok(self.data_abrt_vec = val),
            0x14 => Ok(self.reserved_vec = val),
            0x18 => Ok(self.normal_irq_vec = val),
            0x1C => Ok(self.high_priority_irq_vec = val),
            _ => Err(InvalidAccess),
        }
    }
}
