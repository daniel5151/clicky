use crate::devices::prelude::*;
use relativity::Instant;

#[derive(Debug)]
pub struct Rtc {
    reset_time: Instant
}

impl Rtc {
	pub fn new() -> Rtc {
		Rtc {
            reset_time: Instant::now()
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
                let delta = Instant::now() - self.reset_time;
                Ok(delta.as_millis() as u32)
            },
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, _val: u32) -> MemResult<()> {
        match offset {
            // Written by RetailOS & diagnostics during boot
            // Read in "5 in 1" diagnostic menu, but not decoded
            0x00 => {
                self.reset_time = Instant::now();
                Ok(())
            },
            _ => Err(Unexpected)
        }
    }
}
