pub mod rtc;
pub mod usec_timer;

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

use usec_timer::UsecTimer;

/// PP5020 timer block
#[derive(Debug)]
pub struct Timers {
    usec: UsecTimer,
}

impl Timers {
    pub fn new_hle() -> Timers {
        Timers {
            usec: UsecTimer::new(),
        }
    }
}

impl Device for Timers {
    fn kind(&self) -> &'static str {
        "Timers"
    }

    fn probe(&self, offset: u32) -> Probe {
        match offset {
            0x10..=0x13 => Probe::from_device(&self.usec, offset - 0x10),
            _ => Probe::Unmapped,
        }
    }
}

impl Memory for Timers {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x10..=0x13 => self.usec.r32(offset - 0x10),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x10..=0x13 => self.usec.w32(offset - 0x10, val),
            _ => Err(Unexpected),
        }
    }
}
