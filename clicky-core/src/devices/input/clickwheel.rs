use crate::devices::prelude::*;

use crate::devices::input::{Controls, OptoDevice};
use crate::signal::{self, gpio};

#[derive(Debug)]
pub struct ClickWheel {
    controls: Option<Controls<signal::Slave>>,
    hold: Option<gpio::Receiver>,
}

impl ClickWheel {
    pub fn new() -> ClickWheel {
        ClickWheel {
            controls: None,
            hold: None,
        }
    }

    pub fn register_controls(&mut self, controls: Controls<signal::Slave>, hold: gpio::Receiver) {
        self.controls = Some(controls);
        self.hold = Some(hold);
    }
}

impl OptoDevice for ClickWheel {
    fn read_status(&mut self, command: Option<u32>) -> MemResult<u32> {
        let (controls, hold) = match (&self.controls, &self.hold) {
            (Some(controls), Some(hold)) => (controls, hold),
            _ => return Err(Fatal("no controls registered with the clickwheel".into())),
        };

        let buttons = *0u32
            .set_bit(0, controls.action.asserted())
            .set_bit(1, controls.right.asserted())
            .set_bit(2, controls.left.asserted())
            .set_bit(3, controls.down.asserted())
            .set_bit(4, controls.up.asserted());

        // iPod 4G bootloader requests `0x023a` and looks for that
        // signature in bits 31 + 0..15, having masked out 16..30; it
        // then takes the buttons from bits 16..20. Rockbox knows this
        // format to, see `(status & 0x8000FFFF) == 0x8000023A` in
        // button-clickwheel.c (even though Rockbox only checks for
        // this value for S5L iPods, not PP ones)
        if matches!(command, Some(cmd) if cmd & 0xffff == 0x023a) {
            return Ok(0x8000_023a | (buttons << 16));
        }

        Ok(*0u32
            .set_bits(0..=7, if hold.is_high() { 0x1a } else { 0 }) // 0x1a, or 0 if hold is engaged
            .set_bits(8..=12, buttons)
            .set_bits(16..=22, *controls.wheel.1.lock().unwrap() as u32)
            .set_bit(30, true) // FIXME: don't always return clickwheel active?
            .set_bit(31, hold.is_high())) // set unless hold switch is engaged
    }
}
