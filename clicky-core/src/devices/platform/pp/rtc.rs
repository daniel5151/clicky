use crate::devices::prelude::*;
use chrono::{Datelike, Local, Timelike};

#[derive(Debug)]
pub struct Rtc {

}

impl Rtc {
	pub fn new() -> Rtc {
		Rtc {}
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
                let now = Local::now();
                let packed = (now.second() & 0x3f)
                    | ((now.minute() & 0x3f) << 8)
                    | ((now.hour() & 0x3f) << 16)
                    | ((now.day() & 0x3f) << 24);
                Ok(packed)
            }
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, _offset: u32, _val: u32) -> MemResult<()> {
        Err(Unexpected)
    }
}
