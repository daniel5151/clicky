use crate::devices::prelude::*;

use super::CpuId;

/// PP5020 inter-processor Mailbox.
#[derive(Debug)]
pub struct Mailbox {
    selected_core: CpuId,
    cpu_irq: irq::Sender,
    cop_irq: irq::Sender,

    shared_bits: u32,
}

impl Mailbox {
    pub fn new(cpu_irq: irq::Sender, cop_irq: irq::Sender) -> Mailbox {
        Mailbox {
            selected_core: CpuId::Cpu,
            cpu_irq,
            cop_irq,

            shared_bits: 0,
        }
    }

    pub fn set_cpuid(&mut self, cpuid: CpuId) {
        self.selected_core = cpuid;
    }
}

impl Device for Mailbox {
    fn kind(&self) -> &'static str {
        "Mailbox"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "Status",
            0x04 => "Set",
            0x08 => "Clear",
            0x0c => "?",
            0x10..=0x1f => "<CPU Queue>",
            0x20..=0x2f => "<COP Queue>",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for Mailbox {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => Err(StubRead(Warn, {
                // notice how the IRQ for the _selected_ core is asserted?
                match self.selected_core {
                    CpuId::Cpu => self.cpu_irq.clear(),
                    CpuId::Cop => self.cop_irq.clear(),
                }

                self.shared_bits
            })),
            0x04 => Err(InvalidAccess),
            0x08 => Err(InvalidAccess),
            0x0c => Err(Unimplemented),
            0x10..=0x1f => Err(Unimplemented),
            0x20..=0x2f => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        macro_rules! fire_irq {
            () => {
                // notice how the IRQ for the _other_ core is asserted?
                match self.selected_core {
                    CpuId::Cpu => self.cop_irq.assert(),
                    CpuId::Cop => self.cpu_irq.assert(),
                }
            };
        }

        match offset {
            0x00 => Err(InvalidAccess),
            0x04 => Err(StubWrite(Warn, {
                self.shared_bits |= val;
                fire_irq!()
            })),
            0x08 => Err(StubWrite(Warn, {
                self.shared_bits &= !val;
                fire_irq!()
            })),
            0x0c => Err(Unimplemented),
            0x10..=0x1f => Err(StubWrite(Error, ())),
            0x20..=0x2f => Err(StubWrite(Error, ())),
            _ => Err(Unexpected),
        }
    }
}
