use crate::devices::prelude::*;

use std::time::Instant;

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
        "Microsecond Timer"
    }

    fn probe(&self, offset: u32) -> Probe {
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
                let elapsed_as_micros = elapsed.as_micros() as u32;
                // Reading the timer value in a tight loop could result in a delta time of 0
                if elapsed_as_micros != 0 {
                    self.val = self.val.wrapping_add(elapsed_as_micros);
                    self.last = now;
                }

                Ok(self.val)
            }
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
