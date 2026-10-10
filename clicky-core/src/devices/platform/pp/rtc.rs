use std::time::Duration;

use crate::devices::prelude::*;
use relativity::Instant;

#[derive(Debug)]
pub struct Rtc {
    reset_time: Instant,
    reset_val: Duration,
}

impl Rtc {
    pub fn new() -> Rtc {
        Rtc {
            reset_time: Instant::now(),
            reset_val: Duration::from_secs(0),
        }
    }
}

impl Device for Rtc {
    fn kind(&self) -> &'static str {
        "RTC"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "RTC",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for Rtc {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => {
                let delta = Instant::now() - self.reset_time + self.reset_val;
                let secs = delta.as_secs();
                let s = (secs % 60) as u32;
                let m = (secs / 60 % 60) as u32;
                let h = (secs / 3600 % 24) as u32;
                let d = (secs / 86400 % 64) as u32;
                Ok(d << 24 | h << 16 | m << 8 | s)
            },
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            // Written by RetailOS & diagnostics during boot
            // Read in "5 in 1" diagnostic menu, but not decoded
            0x00 => {
                let s = (val & 0x3f) as u64;
                let m = (val >> 8 & 0x3f) as u64;
                let h = (val >> 16 & 0x1f) as u64;
                let d = (val >> 24 & 0x3f) as u64;
                self.reset_time = Instant::now();
                self.reset_val = Duration::from_secs(((d * 24 + h) * 60 + m) * 60 + s);
                Ok(())
            },
            _ => Err(Unexpected)
        }
    }
}
