use crate::devices::i2c::prelude::*;

#[derive(Debug)]
pub struct Wm8731 {
    last_op_was_write: bool,
    register: Option<u8>,
}

impl Wm8731 {
    pub fn new() -> Wm8731 {
        Wm8731 {
            last_op_was_write: false,
            register: None,
        }
    }
}

impl Device for Wm8731 {
    fn kind(&self) -> &'static str {
        "Wolfson WM8731"
    }

    fn probe(&self, _offset: u32) -> Probe {
        let reg = match self.register {
            Some(reg) => reg,
            None => return Probe::Register("<no register selected>"),
        };

        let reg_name = match reg {
            0x00 => "Left Line In",
            0x01 => "Right Line In",
            0x02 => "Left Headphone Out",
            0x03 => "Right Headphone Out",
            0x04 => "Analogue Audio Path Control",
            0x05 => "Digital Audio Path Control",
            0x06 => "Power Down Control",
            0x07 => "Digital Audio Interface Format",
            0x08 => "Sampling Control",
            0x09 => "Active Control",
            0x0F => "Reset",
            _ => "<invalid>",
        };
        Probe::Register(reg_name)
    }
}

impl I2CDevice for Wm8731 {
    fn read(&mut self) -> MemResult<u8> {
        self.last_op_was_write = false;
        Err(Unexpected) // Write-only
    }

    fn write(&mut self, data: u8) -> MemResult<()> {
        if !self.last_op_was_write {
            self.register = None; // reset the register
        }
        self.last_op_was_write = true;

        match self.register {
            // 7-bit register address, followed by the 9th (MSB) data bit
            None => Ok(self.register = Some(data >> 1)),
            Some(reg) => match reg {
                0x00..=0x09 | 0x0F => Err(StubWrite(Trace, ())),
                _ => Err(Unimplemented),
            },
        }
    }

    fn write_done(&mut self) -> MemResult<()> {
        self.last_op_was_write = false;
        Ok(())
    }
}
