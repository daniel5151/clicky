use crate::devices::prelude::*;

use crate::devices::pwm::PwmSink;

// TRM says PWM frequency is the device clock frequency (max 48 MHz)
// divided by 256. I can't check the exact value of it, but:
// - the 4G iPod MLB has a 24 MHz xtal next to the PortalPlayer chip
// - Rockbox's measurements in piezo.c seem to lean towards
//   22 MHz
// This value isn't critical, so let's go with 24 MHz.
const PWM_CLOCK_HZ: f32 = 24_000_000.0;

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

    /// Output frequency in Hz.
    ///
    /// Tegra's PWM steps the duty cycle over 256 counts, so one output period
    /// is `(scale + 1) * 256` input clocks.
    fn frequency(&self) -> f32 {
        PWM_CLOCK_HZ / ((self.scale as f32 + 1.0) * 256.0)
    }
}

// Looks faily similar to Tegra's PWM controller: https://github.com/torvalds/linux/blob/master/drivers/pwm/pwm-tegra.c
// See also "Tegra 4 Technical Reference Manual", Section 39.2 PWM Registers
#[derive(Debug)]
pub struct PWMCon {
    channels: [PWMConfiguration; 4],
    sinks: [Option<Box<dyn PwmSink>>; 4],
}

impl PWMCon {
    pub fn new() -> PWMCon {
        PWMCon {
            channels: [PWMConfiguration::new(); 4],
            sinks: Default::default(),
        }
    }

    pub fn attach(&mut self, channel: usize, sink: Box<dyn PwmSink>) {
        self.sinks[channel] = Some(sink);
    }

    fn write_channel(&mut self, channel: usize, val: u32) {
        self.channels[channel].write(val);

        let cfg = self.channels[channel];

        trace!(
            target: "PWM",
            "ch{}: enabled={} duty={} scale={} -> {:.1} Hz",
            channel,
            cfg.enabled,
            cfg.duty,
            cfg.scale,
            cfg.frequency()
        );

        if let Some(sink) = &mut self.sinks[channel] {
            // Tegra 2 TRM 20.2.1: "N = N/256 Pulse high". Unconfirmed on the
            // PP5020; Rockbox only ever writes 0x80.
            let duty = f32::from(cfg.duty) / 256.0;

            sink.update(cfg.enabled, cfg.frequency(), duty);
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
            0x00 => Ok(self.write_channel(0, val)),
            0x10 => Ok(self.write_channel(1, val)),
            0x20 => Ok(self.write_channel(2, val)),
            0x30 => Ok(self.write_channel(3, val)),
            _ => Err(Unexpected),
        }
    }
}
