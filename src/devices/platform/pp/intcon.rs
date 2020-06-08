use bit_field::BitField;

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};
use crate::signal::irq;

#[derive(Debug, Default)]
struct IntConCpuRegs {
    irq_stat: u32,
    fiq_stat: u32,
    enabled: u32,
    priority: u32,
}

/// Half of the PP5020 Interrupt Controller.
#[derive(Debug, Default)]
struct IntCon32 {
    label: &'static str,
    irqs: [Option<irq::Reciever>; 32],

    cpu: IntConCpuRegs,
    cop: IntConCpuRegs,
    int_stat: u32,
    int_forced_stat: u32,
    int_forced_set: u32,
    int_forced_clr: u32,
}

impl IntCon32 {
    pub fn new(label: &'static str) -> IntCon32 {
        IntCon32 {
            label,
            irqs: Default::default(),

            cpu: IntConCpuRegs::default(),
            cop: IntConCpuRegs::default(),
            int_stat: 0,
            int_forced_stat: 0,
            int_forced_set: 0,
            int_forced_clr: 0,
        }
    }

    /// Register a device IRQ to a specific index.
    ///
    /// Returns `&mut self` to support chaining registrations.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= 32`
    pub fn register(&mut self, idx: usize, irq: irq::Reciever) -> &mut Self {
        assert!(idx < 32, "idx must be less than 32");
        self.irqs[idx] = Some(irq);
        self
    }

    /// Iterate through `self.irqs`, updating registers accordingly
    fn update_regs(&mut self) {
        for (i, irq) in self.irqs.iter().enumerate() {
            let irq = match irq {
                Some(irq) => irq,
                None => continue,
            };

            let status = irq.asserted();

            for cpu in [&mut self.cpu, &mut self.cop].iter_mut() {
                let enabled = cpu.enabled.get_bit(i);
                let is_fiq = cpu.priority.get_bit(i);
                cpu.fiq_stat.set_bit(i, enabled && is_fiq && status);
                cpu.irq_stat.set_bit(i, enabled && !is_fiq && status);
            }
        }

        // TODO: look into the "forced interrupt" functionality
    }

    pub fn check_fiq(&mut self, is_cop: bool) -> bool {
        self.update_regs();
        let cpu = match is_cop {
            true => &mut self.cop,
            false => &mut self.cpu,
        };
        cpu.enabled & cpu.fiq_stat != 0
    }

    pub fn check_irq(&mut self, is_cop: bool) -> bool {
        self.update_regs();
        let cpu = match is_cop {
            true => &mut self.cop,
            false => &mut self.cpu,
        };
        cpu.enabled & cpu.irq_stat != 0
    }
}

impl Device for IntCon32 {
    fn kind(&self) -> &'static str {
        "Interrupt Controller"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "CpuIrqStat",
            0x04 => "CopIrqStat",
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

impl Memory for IntCon32 {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => Ok(self.cpu.irq_stat),
            0x04 => Ok(self.cop.irq_stat),
            0x08 => Ok(self.cpu.fiq_stat),
            0x0c => Ok(self.cop.fiq_stat),

            0x10 => Err(Unimplemented),
            0x14 => Err(Unimplemented),
            0x18 => Err(Unimplemented),
            0x1c => Err(Unimplemented),

            0x20 => Ok(self.cpu.enabled),
            0x24 => Err(InvalidAccess),
            0x28 => Err(InvalidAccess),
            0x2c => Err(Unimplemented),

            0x30 => Ok(self.cop.enabled),
            0x34 => Err(InvalidAccess),
            0x38 => Err(InvalidAccess),
            0x3c => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => Err(InvalidAccess),
            0x04 => Err(InvalidAccess),
            0x08 => Err(InvalidAccess),
            0x0c => Err(InvalidAccess),

            0x10 => Err(Unimplemented),
            0x14 => Err(Unimplemented),
            0x18 => Err(Unimplemented),
            0x1c => Err(Unimplemented),

            0x20 => Err(InvalidAccess),
            0x24 => Ok(self.cpu.enabled |= val),
            0x28 => Ok(self.cpu.enabled &= !val),
            0x2c => Ok(self.cpu.priority = val),

            0x30 => Err(InvalidAccess),
            0x34 => Ok(self.cop.enabled |= val),
            0x38 => Ok(self.cop.enabled &= !val),
            0x3c => Ok(self.cop.priority = val),
            _ => Err(Unexpected),
        }
    }
}

/// PP5020 Interrupt Controller
#[derive(Debug, Default)]
pub struct IntCon {
    lo: IntCon32,
    hi: IntCon32,
}

impl IntCon {
    pub fn new() -> IntCon {
        IntCon {
            lo: IntCon32::new("lo"),
            hi: IntCon32::new("hi"),
        }
    }

    /// Register a device IRQ to a specific index.
    ///
    /// Returns `&mut self` to support chaining registrations.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= 64 || idx == 30` (IRQ 30 is a "master toggle"
    /// for all hi IRQs, i.e: IRQs with `idx >= 32`)
    pub fn register(&mut self, idx: usize, irq: irq::Reciever) -> &mut Self {
        assert!(idx < 64, "idx must be less than 64");
        assert!(idx != 30, "idx 30 is reserved for internal use");
        if idx < 32 {
            self.lo.register(idx, irq);
        } else {
            self.hi.register(idx - 32, irq);
        }

        self
    }

    fn hi_enabled(&self, is_cop: bool) -> bool {
        match is_cop {
            true => self.lo.cop.enabled.get_bit(30),
            false => self.lo.cpu.enabled.get_bit(30),
        }
    }

    /// Check if an IRQ is being requested, updating registers appropriately.
    pub fn check_irq(&mut self, is_cop: bool) -> bool {
        self.lo.check_irq(is_cop) || (self.hi_enabled(is_cop) && self.hi.check_irq(is_cop))
    }

    /// Check if a FIQ is being requested, updating registers appropriately.
    pub fn check_fiq(&mut self, is_cop: bool) -> bool {
        self.lo.check_fiq(is_cop) || (self.hi_enabled(is_cop) && self.hi.check_fiq(is_cop))
    }
}

impl Device for IntCon {
    fn kind(&self) -> &'static str {
        "<intcon manager>"
    }

    fn probe(&self, offset: u32) -> Probe {
        match offset {
            0x000..=0x0ff => Probe::from_device(&self.lo, offset),
            0x100..=0x1ff => Probe::from_device(&self.hi, offset - 0x100),
            _ => Probe::Unmapped,
        }
    }
}

impl Memory for IntCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x000..=0x0ff => self.lo.r32(offset),
            0x100..=0x1ff => self.hi.r32(offset - 0x100),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x000..=0x0ff => self.lo.w32(offset, val),
            0x100..=0x1ff => self.hi.w32(offset - 0x100, val),
            _ => Err(Unexpected),
        }
    }
}
