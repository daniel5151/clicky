use crate::devices::prelude::*;

pub mod cfg_timer;
pub mod rtc;
pub mod usec_timer;

use cfg_timer::CfgTimer;
use usec_timer::UsecTimer;

/// PP5020 timer block
#[derive(Debug)]
pub struct Timers {
    usec: UsecTimer,
    cfg_timer1: CfgTimer,
    cfg_timer2: CfgTimer,
}

impl Timers {
    pub fn new() -> Timers {
        Timers {
            usec: UsecTimer::new(),
            cfg_timer1: CfgTimer::new("1"),
            cfg_timer2: CfgTimer::new("2"),
        }
    }
}

impl Device for Timers {
    fn kind(&self) -> &'static str {
        "Timers"
    }

    fn probe(&self, offset: u32) -> Probe {
        match offset {
            0x00..=0x07 => Probe::from_device(&self.cfg_timer1, offset),
            0x08..=0x0f => Probe::from_device(&self.cfg_timer2, offset - 0x08),
            0x10..=0x13 => Probe::from_device(&self.usec, offset - 0x10),
            _ => Probe::Unmapped,
        }
    }
}

impl Memory for Timers {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00..=0x07 => self.cfg_timer1.r32(offset),
            0x08..=0x0f => self.cfg_timer2.r32(offset - 0x08),
            0x10..=0x13 => self.usec.r32(offset - 0x10),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00..=0x07 => self.cfg_timer1.w32(offset, val),
            0x08..=0x0f => self.cfg_timer2.w32(offset - 0x08, val),
            0x10..=0x13 => self.usec.w32(offset - 0x10, val),
            _ => Err(Unexpected),
        }
    }
}
