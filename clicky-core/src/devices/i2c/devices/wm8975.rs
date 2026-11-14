use crate::devices::i2c::prelude::*;

#[derive(Debug)]
pub struct Wm8975 {
    last_op_was_write: bool,
    register: Option<u8>,
    inner: Wm8975Impl,
}

impl Wm8975 {
    pub fn new() -> Wm8975 {
        Wm8975 {
            last_op_was_write: false,
            register: None,
            inner: Wm8975Impl::new(),
        }
    }
}

impl Device for Wm8975 {
    fn kind(&self) -> &'static str {
        "Wolfson WM8975"
    }

    fn probe(&self, _offset: u32) -> Probe {
        let reg = match self.register {
            Some(reg) => reg,
            None => return Probe::Register("<no register selected>"),
        };
        self.inner.probe(reg)
    }
}

impl I2CDevice for Wm8975 {
    fn read(&mut self) -> MemResult<u8> {
        self.last_op_was_write = false;

        match self.register {
            None => Err(Fatal("no register specified for read".into())),
            Some(reg) => self.inner.read(reg),
        }
    }

    fn write(&mut self, data: u8) -> MemResult<()> {
        if !self.last_op_was_write {
            self.register = None; // reset the register
        }
        self.last_op_was_write = true;

        match self.register {
            None => Ok(self.register = Some(data >> 1)),
            Some(reg) => self.inner.write(reg, data),
        }
    }

    fn write_done(&mut self) -> MemResult<()> {
        self.last_op_was_write = false;
        Ok(())
    }
}

#[derive(Debug)]
struct Wm8975Impl {
    reg_gpio: u8,
}

impl Wm8975Impl {
    fn new() -> Wm8975Impl {
        Wm8975Impl { reg_gpio: 0 }
    }

    fn probe(&self, reg: u8) -> Probe {
        let reg_name = match reg {
            0x00 => "Software Reset",
            0x01 => "Power management 1",
            0x02 => "Power management 2",
            0x03 => "Power management 3",
            0x04 => "Audio interface",
            0x05 => "Companding ctrl",
            0x06 => "Clock Gen ctrl",
            0x07 => "Additional ctrl",
            0x08 => "GPIO",
            0x0A => "DAC Control",
            0x0B => "DAC digital Vol",
            0x0E => "ADC Control",
            0x0F => "ADC Digital Vol",
            0x10 => "Undocumented 0x10",
            0x12 => "EQ1 - low shelf",
            0x13 => "EQ2 - peak 1",
            0x14 => "EQ3 - peak 2",
            0x15 => "EQ4 - peak 3",
            0x16 => "EQ5 - high shelf",
            0x18 => "DAC Limiter 1",
            0x19 => "DAC Limiter 2",
            0x1A => "Undocumented 0x1A",
            0x1B => "Notch Filter 1",
            0x1C => "Notch Filter 2",
            0x1D => "Notch Filter 2",
            0x1E => "Notch Filter 2",
            0x20 => "ALC control 1",
            0x21 => "ALC control 2",
            0x22 => "ALC control 3",
            0x23 => "Noise Gate",
            0x24 => "PLL N",
            0x25 => "PLL K 1",
            0x26 => "PLL K 2",
            0x27 => "PLL K 3",
            0x28 => "Attenuation ctrl",
            0x2C => "Input ctrl",
            0x2D => "INP PGA gain ctrl",
            0x2F => "ADC Boost ctrl",
            0x31 => "Output ctrl",
            0x32 => "SPK mixer ctrl",
            0x33 => "SPK volume ctrl",
            0x38 => "MONO mixer ctrl",
            _ => "<invalid>",
        };
        Probe::Register(reg_name)
    }

    fn read(&mut self, _reg: u8) -> MemResult<u8> {
        Err(Unimplemented)
    }

    fn write(&mut self, reg: u8, _data: u8) -> MemResult<()> {
        match reg {
            0x05 => Err(StubWrite(Trace, ())),
            0x07 => Err(StubWrite(Trace, ())),
            0x08 => Err(StubWrite(Error, ())),
            0x0F => Err(StubWrite(Trace, ())),
            0x10 | 0x1A => Err(StubWrite(Error, ())), // Undocumented registers
            0x18..=0x19 => Ok(()), // Ignore DAC Limiter registers (not needed)
            0x1B..=0x1E => Err(StubWrite(Trace, ())), // Ignore Notch Filter (might be emulated later)
            0x20..=0x22 => Err(StubWrite(Trace, ())), // Ignore ALC control
            0x24..=0x27 => Err(StubWrite(Trace, ())), // Ignore PLL registers
            _ => Err(Unimplemented),
        }
    }
}
