use crate::devices::prelude::*;

/// Template for creating new devices.
#[derive(Debug)]
pub struct CfgTimer {
    label: &'static str,

    counter: u32,
    unknown_cfg: bool,
    repeat: bool,
    enable: bool,

    val: u32,
}

impl CfgTimer {
    pub fn new(label: &'static str) -> CfgTimer {
        CfgTimer {
            label,
            counter: 0,
            unknown_cfg: false,
            repeat: false,
            enable: false,
            val: 0,
        }
    }
}

impl Device for CfgTimer {
    fn kind(&self) -> &'static str {
        "Configurable Timer"
    }

    fn label(&self) -> Option<&'static str> {
        Some(self.label)
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x0 => "Config",
            0x4 => "Val / Clear IRQ",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for CfgTimer {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0 => Err(StubRead(Warn, {
                *0u32
                    .set_bits(0..=28, self.counter)
                    .set_bit(29, self.unknown_cfg)
                    .set_bit(30, self.repeat)
                    .set_bit(31, self.enable)
            })),
            0x4 => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x0 => Err(StubWrite(Warn, {
                self.counter = val.get_bits(0..=28);
                self.unknown_cfg = val.get_bit(29);
                self.repeat = val.get_bit(30);
                self.enable = val.get_bit(31);
            })),
            0x4 => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }
}
