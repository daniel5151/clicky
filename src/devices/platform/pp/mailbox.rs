use log::Level::*;

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// PP5020 inter-processor Mailbox.
#[derive(Debug)]
pub struct Mailbox {
    shared_bits: u32,
}

impl Mailbox {
    pub fn new() -> Mailbox {
        Mailbox { shared_bits: 0 }
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
            0x00 => Err(StubRead(Warn, self.shared_bits)),
            0x04 => Err(InvalidAccess),
            0x08 => Err(InvalidAccess),
            0x0c => Err(Unimplemented),
            0x10..=0x1f => Err(Unimplemented),
            0x20..=0x2f => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => Err(InvalidAccess),
            0x04 => Err(StubWrite(Warn, self.shared_bits |= val)),
            0x08 => Err(StubWrite(Warn, self.shared_bits &= !val)),
            0x0c => Err(Unimplemented),
            0x10..=0x1f => Err(StubWrite(Error, ())),
            0x20..=0x2f => Err(StubWrite(Error, ())),
            _ => Err(Unexpected),
        }
    }
}
