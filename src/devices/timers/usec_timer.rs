use std::time::Instant;

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// 32 bit timer which ticks every usec.
#[derive(Debug)]
pub struct UsecTimer {
    val: u32,
    last: Instant,
}

impl UsecTimer {
    pub fn new() -> UsecTimer {
        UsecTimer {
            val: 0,
            last: Instant::now(),
        }
    }
}

impl Device for UsecTimer {
    fn kind(&self) -> &'static str {
        "UsecTimer"
    }

    fn probe(&self, offset: u32) -> Probe<'_> {
        let reg = match offset {
            0x0 => "Val",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for UsecTimer {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0 => {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last);
                self.val = self.val.wrapping_add(elapsed.as_micros() as u32);
                self.last = now;
                Ok(self.val)
            }
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        let _ = val;

        match offset {
            0x0 => Err(InvalidAccess),
            _ => Err(Unexpected),
        }
    }
}
