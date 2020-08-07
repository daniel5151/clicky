use crate::devices::prelude::*;

pub struct IntStatus {
    pub irq: bool,
    pub fiq: bool,
}

macro_rules! impl_and_or {
    ($op:tt, $kind:ident, $fn:ident) => {
        impl core::ops::$kind for IntStatus {
            type Output = Self;
            fn $fn(self, other: Self) -> Self {
                IntStatus {
                    irq: self.irq $op other.irq,
                    fiq: self.fiq $op other.fiq,
                }
            }
        }

        impl core::ops::$kind<bool> for IntStatus {
            type Output = Self;
            fn $fn(self, other: bool) -> Self {
                IntStatus {
                    irq: self.irq $op other,
                    fiq: self.fiq $op other,
                }
            }
        }
    };
}

impl_and_or!(&, BitAnd, bitand);
impl_and_or!(|, BitOr, bitor);

#[derive(Debug, Default)]
struct IntConCpuRegs {
    irq_stat: u32,
    fiq_stat: u32,
    enabled: u32,
    priority: u32,
}

#[derive(Debug)]
enum IrqKind {
    Unregistered,
    Shared(irq::Reciever),
    // i.e: mailbox
    CoreSpecific {
        cpu_irq: irq::Reciever,
        cop_irq: irq::Reciever,
    },
}

impl Default for IrqKind {
    fn default() -> IrqKind {
        IrqKind::Unregistered
    }
}

/// Half of the PP5020 Interrupt Controller.
#[derive(Debug, Default)]
struct IntCon32 {
    label: &'static str,
    irqs: [IrqKind; 32],

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
        self.irqs[idx] = IrqKind::Shared(irq);
        self
    }

    /// Register core specific device IRQs to a specific index.
    ///
    /// Returns `&mut self` to support chaining registrations.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= 32`
    pub fn register_core_specific(
        &mut self,
        idx: usize,
        cpu_irq: irq::Reciever,
        cop_irq: irq::Reciever,
    ) -> &mut Self {
        assert!(idx < 32, "idx must be less than 32");
        self.irqs[idx] = IrqKind::CoreSpecific { cpu_irq, cop_irq };
        self
    }

    /// Iterate through `self.irqs`, updating registers accordingly
    fn update_regs(&mut self) {
        for (i, irq) in self.irqs.iter().enumerate() {
            let (cpu_irq, cop_irq) = match irq {
                IrqKind::Unregistered => continue,
                IrqKind::Shared(irq) => (irq, irq),
                IrqKind::CoreSpecific { cpu_irq, cop_irq } => (cpu_irq, cop_irq),
            };

            for (cpu, irq) in [(&mut self.cpu, cpu_irq), (&mut self.cop, cop_irq)].iter_mut() {
                let status = irq.asserted();
                let enabled = cpu.enabled.get_bit(i);
                let is_fiq = cpu.priority.get_bit(i);
                cpu.fiq_stat.set_bit(i, enabled && is_fiq && status);
                cpu.irq_stat.set_bit(i, enabled && !is_fiq && status);
            }
        }

        // TODO: look into the "forced interrupt" functionality
    }

    pub fn interrupt_status(&mut self) -> (IntStatus, IntStatus) {
        self.update_regs();
        (
            IntStatus {
                irq: self.cpu.enabled & self.cpu.irq_stat != 0,
                fiq: self.cpu.enabled & self.cpu.fiq_stat != 0,
            },
            IntStatus {
                irq: self.cop.enabled & self.cop.irq_stat != 0,
                fiq: self.cop.enabled & self.cop.fiq_stat != 0,
            },
        )
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

            // based off some random dude's comment in an IRC chat from 2008
            // https://www.rockbox.org/irc/log-20080219
            0x44 => "(?) <serial related?>",
            0x4c => "(?) <serial related?>",
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
            0x2c => Ok(self.cpu.priority),

            0x30 => Ok(self.cop.enabled),
            0x34 => Err(InvalidAccess),
            0x38 => Err(InvalidAccess),
            0x3c => Ok(self.cop.priority),

            0x44 => Err(Unimplemented),
            0x4c => Err(Unimplemented),
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
            0x1c => Err(StubWrite(Error, ())), // TODO: figure out what this does

            0x20 => Err(InvalidAccess),
            0x24 => Ok(self.cpu.enabled |= val),
            0x28 => Ok(self.cpu.enabled &= !val),
            0x2c => Ok(self.cpu.priority = val),

            0x30 => Err(InvalidAccess),
            0x34 => Ok(self.cop.enabled |= val),
            0x38 => Ok(self.cop.enabled &= !val),
            0x3c => Ok(self.cop.priority = val),

            0x44 => Err(StubWrite(Error, ())),
            0x4c => Err(StubWrite(Error, ())),
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

    /// Register core specific device IRQs to a specific index.
    ///
    /// Returns `&mut self` to support chaining registrations.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= 64 || idx == 30` (IRQ 30 is a "master toggle"
    /// for all hi IRQs, i.e: IRQs with `idx >= 32`)
    pub fn register_core_specific(
        &mut self,
        idx: usize,
        cpu_irq: irq::Reciever,
        cop_irq: irq::Reciever,
    ) -> &mut Self {
        assert!(idx < 64, "idx must be less than 64");
        assert!(idx != 30, "idx 30 is reserved for internal use");
        if idx < 32 {
            self.lo.register_core_specific(idx, cpu_irq, cop_irq);
        } else {
            self.hi.register_core_specific(idx - 32, cpu_irq, cop_irq);
        }

        self
    }

    fn hi_enabled(&self) -> (bool, bool) {
        (
            self.lo.cpu.enabled.get_bit(30),
            self.lo.cop.enabled.get_bit(30),
        )
    }

    /// Check if an IRQ/FIQ is being requested on the (cpu, cop)
    pub fn interrupt_status(&mut self) -> (IntStatus, IntStatus) {
        let (lo_cpu, lo_cop) = self.lo.interrupt_status();
        let (hien_cpu, hien_cop) = self.hi_enabled();
        let (hi_cpu, hi_cop) = self.hi.interrupt_status();

        (
            (lo_cpu | (hi_cpu & hien_cpu)),
            (lo_cop | (hi_cop & hien_cop)),
        )
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
