use bit_field::BitField;

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// iPod Piezo speaker.
#[derive(Debug)]
pub struct Piezo {
    control: u32,
}

impl Piezo {
    fn on_update_piezo(&self) {
        // TODO: actually output some sound
        let _freq = self.control.get_bits(0..=15) as u16;
        let _form = self.control.get_bits(16..=23) as u8;
        let _enabled = self.control.get_bit(31);
    }

    pub fn new() -> Piezo {
        Piezo { control: 0 }
    }
}

impl Device for Piezo {
    fn kind(&self) -> &'static str {
        "iPod Piezo"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x0 => "Control",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for Piezo {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0 => Ok(self.control),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x0 => {
                self.control = val;
                self.on_update_piezo();
                Ok(())
            }
            _ => Err(Unexpected),
        }
    }
}
