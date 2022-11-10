use crate::devices::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct PWMConfiguration {
    enabled: bool,
    duty: u8,
    scale: u16,
}

impl PWMConfiguration {
    pub fn new() -> PWMConfiguration {
        PWMConfiguration {
            enabled: false,
            duty: 0,
            scale: 0,
        }
    }

    fn read(&self) -> u32 {
        *0u32
            .set_bit(31, self.enabled)
            .set_bits(16..=23, self.duty as u32)
            .set_bits(0..=12, self.scale as u32)
    }

    fn write(&mut self, reg: u32) {
        self.enabled = reg.get_bit(31);
        self.duty = reg.get_bits(16..=23) as u8;
        self.scale = reg.get_bits(0..=12) as u16;
    }
}

// Looks faily similar to Tegra's PWM controller: https://github.com/torvalds/linux/blob/master/drivers/pwm/pwm-tegra.c
// See also "Tegra 4 Technical Reference Manual", Section 39.2 PWM Registers
#[derive(Debug)]
pub struct PWMCon {
    channels: [PWMConfiguration; 4],
}

impl PWMCon {
    pub fn new() -> PWMCon {
        PWMCon {
            channels: [PWMConfiguration::new(); 4],
        }
    }
}

impl Device for PWMCon {
    fn kind(&self) -> &'static str {
        "PWM Controller"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "Channel 0 configuration (piezo)",
            0x10 => "Channel 1 configuration (LCD backlight)",
            0x20 => "Channel 2 configuration",
            0x30 => "Channel 3 configuration",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for PWMCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => Ok(self.channels[0].read()),
            0x10 => Ok(self.channels[1].read()),
            0x20 => Ok(self.channels[2].read()),
            0x30 => Ok(self.channels[3].read()),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => Ok(self.channels[0].write(val)),
            0x10 => Ok(self.channels[1].write(val)),
            0x20 => Ok(self.channels[2].write(val)),
            0x30 => Ok(self.channels[3].write(val)),
            _ => Err(Unexpected),
        }
    }
}
