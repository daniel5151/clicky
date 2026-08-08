use crate::devices::prelude::*;

use super::CpuId;

const MAILBOX_QUEUE_SIZE: usize = 4;

/// PP5020 inter-processor Mailbox.
#[derive(Debug)]
pub struct Mailbox {
    selected_core: CpuId,
    cpu_irq: irq::Sender,
    cop_irq: irq::Sender,

    shared_bits: u32,

    cpu_queue: [u32; MAILBOX_QUEUE_SIZE],
    cop_queue: [u32; MAILBOX_QUEUE_SIZE],
}

impl Mailbox {
    pub fn new(cpu_irq: irq::Sender, cop_irq: irq::Sender) -> Mailbox {
        Mailbox {
            selected_core: CpuId::Cpu,
            cpu_irq,
            cop_irq,

            shared_bits: 0,

            cpu_queue: [0; MAILBOX_QUEUE_SIZE],
            cop_queue: [0; MAILBOX_QUEUE_SIZE],
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
            0x00 => Ok({
                // notice how the IRQ for the _selected_ core is asserted?
                match self.selected_core {
                    CpuId::Cpu => self.cpu_irq.clear(),
                    CpuId::Cop => self.cop_irq.clear(),
                }

                self.shared_bits
            }),
            0x04 => Err(InvalidAccess),
            0x08 => Err(InvalidAccess),
            0x0c => Err(Unimplemented),
            0x10..=0x1f => {
                let idx = (offset as usize & 0xf) / 4;
                if idx == 0 {
                    self.cpu_irq.clear();
                }
                Ok(self.cpu_queue[idx])
            },
            0x20..=0x2f => {
                let idx = (offset as usize & 0xf) / 4;
                if idx == 0 {
                    self.cop_irq.clear();
                }
                Ok(self.cop_queue[idx])
            },
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
            0x04 => Ok({
                self.shared_bits |= val;
                fire_irq!()
            }),
            0x08 => Ok({
                self.shared_bits &= !val;
                fire_irq!()
            }),
            0x0c => Err(Unimplemented),
            0x10..=0x1f => {
                let idx = (offset as usize & 0xf) / 4;
                self.cpu_queue[idx] = val;
                if idx == 0 {
                    self.cpu_irq.assert();
                }
                Ok(())
            },
            0x20..=0x2f => {
                let idx = (offset as usize & 0xf) / 4;
                self.cop_queue[idx] = val;
                if idx == 0 {
                    self.cop_irq.assert();
                }
                Ok(())
            },
            _ => Err(Unexpected),
        }
    }
}
