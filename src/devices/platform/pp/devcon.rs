use crate::devices::prelude::*;

/// PP5020 Device Controller.
#[derive(Debug)]
pub struct DevCon {
    reset: [u32; 2],
    enable: [u32; 2],
    clock_source: u32,
    pll_control: u32,
    pll_status: u32,
    cache_priority: u8,
    mystery_i2c: u8,
    mystery: [u32; 1],
}

impl DevCon {
    pub fn new() -> DevCon {
        DevCon {
            reset: [0, 0],
            enable: [0, 0],
            clock_source: 0,
            pll_control: 0,
            pll_status: 0,
            cache_priority: 0,
            mystery_i2c: 0,
            mystery: [0; 1],
        }
    }
}

impl Device for DevCon {
    fn kind(&self) -> &'static str {
        "DevCon"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x04 => "Device Reset 1",
            0x08 => "Device Reset 2",
            0x0c => "Device Enable 1",
            0x10 => "Device Enable 2",
            0x20 => "Clock Source",
            0x34 => "PLL Control",
            0x3c => "PLL Status",
            0x44 => "Cache Priority",
            0xa4 => "(?) I2C related",
            0xc4 => "(?) DMA clock related",
            0xc8 => "?",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for DevCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x04 => Err(StubRead(Error, self.reset[0])),
            0x08 => Err(StubRead(Error, self.reset[1])),
            0x0c => Ok(self.enable[0]),
            0x10 => Ok(self.enable[1]),
            0x20 => Err(StubRead(Error, self.clock_source)),
            0x34 => Err(StubRead(Error, self.pll_control)),
            0x3c => Err(StubRead(Error, self.pll_status)),
            0x44 => Err(StubRead(Error, self.cache_priority as u32)),
            0xa4 => Err(StubRead(Error, self.mystery_i2c as u32)),
            0xc4 => Err(InvalidAccess),
            0xc8 => Err(StubRead(Error, self.mystery[0])),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x04 => Err(StubWrite(Error, self.reset[0] = val)),
            0x08 => Err(StubWrite(Error, self.reset[1] = val)),
            0x0c => Err(StubWrite(Info, self.enable[0] = val)),
            0x10 => Err(StubWrite(Info, self.enable[1] = val)),
            0x20 => Err(StubWrite(Warn, self.clock_source = val)),
            0x34 => Err(StubWrite(Warn, self.pll_control = val)),
            0x3c => Err(StubWrite(Warn, self.pll_status = val)),
            0x44 => Err(StubWrite(Warn, {
                self.cache_priority = val as u8;
                if val > 0xff {
                    return Err(ContractViolation {
                        msg: "wrote non 8-bit value to cache_priority".into(),
                        severity: Error,
                        stub_val: None,
                    });
                }
            })),
            0xa4 => Err(StubWrite(Error, self.mystery_i2c = val as u8)),
            0xc4 => Err(StubWrite(Info, ())),
            0xc8 => Err(StubWrite(Error, self.mystery[0] = val)),
            _ => Err(Unexpected),
        }
    }
}
