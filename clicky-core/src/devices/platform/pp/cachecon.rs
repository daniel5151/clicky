use crate::devices::prelude::*;

/// PP5020 Cache Controller.
#[derive(Debug)]
pub struct CacheCon {
    /// Local exception vector table enable (bit 4)
    pub local_evt: bool,
    /// Cache control enable (bit 1)
    cache_ctrl_enable: bool,
}

impl CacheCon {
    pub fn new() -> CacheCon {
        CacheCon {
            local_evt: false,
            cache_ctrl_enable: false,
        }
    }
}

impl Device for CacheCon {
    fn kind(&self) -> &'static str {
        "Cache Controller"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "Control",
            0x10 => "(?)",
            0x34 => "(?)",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for CacheCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => {
                let val = *0u32
                    .set_bit(4, self.local_evt)
                    .set_bit(1, self.cache_ctrl_enable);
                Err(StubRead(Warn, val))
            }
            0x10 => Err(InvalidAccess),
            0x34 => Err(InvalidAccess),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => {
                self.local_evt = val.get_bit(4);
                self.cache_ctrl_enable = val.get_bit(1);
                Err(StubWrite(Error, ()))
            }
            0x10 => Err(StubWrite(Error, ())),
            0x34 => Err(StubWrite(Error, ())),
            _ => Err(Unexpected),
        }
    }
}
