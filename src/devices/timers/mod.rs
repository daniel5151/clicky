pub mod irq_timer;
pub mod rtc;
pub mod usec_timer;

use crossbeam_channel as chan;

use crate::devices::{Device, Interrupt, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

use irq_timer::IrqTimer;
use usec_timer::UsecTimer;

/// PP5020 timer block
// FIXME: this really shouldn't be in this file...
#[derive(Debug)]
pub struct Timers<I: Interrupt> {
    timer1: IrqTimer<I>,
    timer2: IrqTimer<I>,
    usec: UsecTimer,
}

impl<I: Interrupt> Timers<I> {
    pub fn new_hle(
        interrupt_bus: chan::Sender<(I, bool)>,
        timer1int: I,
        timer2int: I,
    ) -> Timers<I> {
        Timers {
            timer1: IrqTimer::new("1", interrupt_bus.clone(), timer1int),
            timer2: IrqTimer::new("2", interrupt_bus.clone(), timer2int),
            usec: UsecTimer::new(),
        }
    }
}

impl<I: Interrupt> Device for Timers<I> {
    fn kind(&self) -> &'static str {
        "Timers"
    }

    fn probe(&self, offset: u32) -> Probe {
        match offset {
            0x00..=0x07 => Probe::from_device(&self.timer1, offset),
            0x08..=0x0f => Probe::from_device(&self.timer2, offset - 0x8),
            0x10..=0x13 => Probe::from_device(&self.usec, offset - 0x10),
            _ => Probe::Unmapped,
        }
    }
}

impl<I: Interrupt> Memory for Timers<I> {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00..=0x07 => self.timer1.r32(offset),
            0x08..=0x0f => self.timer2.r32(offset - 0x8),
            0x10..=0x13 => self.usec.r32(offset - 0x10),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00..=0x07 => self.timer1.w32(offset, val),
            0x08..=0x0f => self.timer2.w32(offset - 0x8, val),
            0x10..=0x13 => self.usec.w32(offset - 0x10, val),
            _ => Err(Unexpected),
        }
    }
}
