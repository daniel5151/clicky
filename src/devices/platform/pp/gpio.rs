use crate::devices::prelude::*;

use crate::devices::util::arcmutex::ArcMutexDevice;
use crate::signal::{gpio, irq};

/// 8-bit GPIO Port
#[derive(Debug)]
struct GpioPort {
    label: &'static str,

    irq: irq::Sender,
    inputs: [Option<gpio::Reciever>; 8],
    outputs: [Option<gpio::Sender>; 8],

    enable: u8,
    output_enable: u8,
    output_val: u8,
    input_val: u8,
    interrupt_status: u8,
    interrupt_enable: u8,
    interrupt_level: u8,
}

impl GpioPort {
    fn new(irq: irq::Sender, label: &'static str) -> GpioPort {
        GpioPort {
            label,

            irq,
            inputs: Default::default(),
            outputs: Default::default(),

            enable: 0,
            output_enable: 0,
            output_val: 0,
            input_val: 0,
            interrupt_status: 0,
            interrupt_enable: 0,
            interrupt_level: 0, // 0 = irq on falling edge, 1 = irq on rising edge
        }
    }

    /// # Panics
    ///
    /// Panics if `idx >= 8`
    fn register_in(&mut self, idx: usize, signal: gpio::Reciever) -> &mut Self {
        assert!(idx < 8, "idx must be less than 8");
        self.inputs[idx] = Some(signal);
        self
    }

    /// # Panics
    ///
    /// Panics if `idx >= 8`
    fn register_out(&mut self, idx: usize, signal: gpio::Sender) -> &mut Self {
        assert!(idx < 8, "idx must be less than 8");
        self.outputs[idx] = Some(signal);
        self
    }

    fn update(&mut self) {
        // update outputs
        for (i, output) in self.outputs.iter_mut().enumerate() {
            // if the port isn't enabled, don't do anything
            if !self.enable.get_bit(i) || !self.output_enable.get_bit(i) {
                continue;
            }

            // set the output line
            if let Some(output) = output {
                match self.output_val.get_bit(i) {
                    false => output.set_high(),
                    true => output.set_low(),
                }
            }
        }

        // update inputs
        for (i, input) in self.inputs.iter().enumerate() {
            // if the port isn't enabled, don't do anything
            if !self.enable.get_bit(i) {
                continue;
            }

            // check the GPIO line's level
            let level = match input {
                Some(input) => input.is_high(),
                None => continue,
            };

            // set the input level
            let prev_level = self.input_val.get_bit(i);
            self.input_val.set_bit(i, level);

            // update the IRQ status register
            let trigger_irq = match self.interrupt_level.get_bit(i) {
                // falling edge trigger
                false => prev_level && !level,
                // rising edge trigger
                true => !prev_level && level,
            };
            self.interrupt_status.set_bit(i, trigger_irq);
        }

        // check if the IRQ line should be asserted / cleared
        if (self.interrupt_status & self.interrupt_enable) != 0 {
            self.irq.assert()
        } else {
            self.irq.clear()
        }
    }
}

impl Device for GpioPort {
    fn kind(&self) -> &'static str {
        "GPIO Port"
    }

    fn label(&self) -> Option<&'static str> {
        Some(&self.label)
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "Enable",
            0x10 => "OutputEnable",
            0x20 => "OutputVal",
            0x30 => "InputVal",
            0x40 => "IntStatus",
            0x50 => "IntEnable",
            0x60 => "IntLevel",
            0x70 => "IntClear",
            _ => return Probe::Unmapped,
        };
        Probe::Register(reg)
    }
}

impl Memory for GpioPort {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => Ok(self.enable as u32),
            0x10 => Ok(self.output_enable as u32),
            0x20 => Ok(self.output_val as u32),
            0x30 => Ok(self.input_val as u32),
            0x40 => Ok(self.interrupt_status as u32),
            0x50 => Ok(self.interrupt_enable as u32),
            0x60 => Ok(self.interrupt_level as u32),
            0x70 => Err(InvalidAccess),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        // it's an 8-bit interface
        let val = val as u8;

        match offset {
            0x00 => self.enable = val,
            0x10 => self.output_enable = val,
            0x20 => self.output_val = val,
            0x30 => return Err(InvalidAccess),
            0x40 => return Err(InvalidAccess),
            0x50 => self.interrupt_enable = val,
            0x60 => self.interrupt_level = val,
            0x70 => self.interrupt_status &= !val,
            _ => return Err(Unexpected),
        };

        self.update();

        Ok(())
    }
}

/// Block of 4 GPIO ports on the PP5020.
#[derive(Debug)]
pub struct GpioBlock {
    port: [GpioPort; 4],
}

impl GpioBlock {
    pub fn new(irq: irq::Sender, labels: [&'static str; 4]) -> GpioBlock {
        GpioBlock {
            port: [
                GpioPort::new(irq.clone(), labels[0]),
                GpioPort::new(irq.clone(), labels[1]),
                GpioPort::new(irq.clone(), labels[2]),
                GpioPort::new(irq, labels[3]),
            ],
        }
    }

    /// Register a GPIO input signal.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= 32`
    pub fn register_in(&mut self, idx: usize, signal: gpio::Reciever) -> &mut Self {
        assert!(idx < 32, "idx must be less than 32");
        self.port[idx / 8].register_in(idx % 8, signal);
        self
    }

    /// Register a GPIO output signal.
    ///
    /// # Panics
    ///
    /// Panics if `idx >= 32`
    pub fn register_out(&mut self, idx: usize, signal: gpio::Sender) -> &mut Self {
        assert!(idx < 32, "idx must be less than 32");
        self.port[idx / 8].register_out(idx % 8, signal);
        self
    }

    /// Propagate GPIO signal changes through the GPIO controller (triggering an
    /// IRQ if necessary).
    pub fn update(&mut self) {
        for port in self.port.iter_mut() {
            port.update()
        }
    }
}

impl Device for GpioBlock {
    fn kind(&self) -> &'static str {
        "4xGPIO Port Block"
    }

    fn probe(&self, offset: u32) -> Probe {
        let port = (offset / 4) % 4;
        Probe::from_device(&self.port[port as usize], offset - 4 * port)
    }
}

impl Memory for GpioBlock {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        let port = (offset / 4) % 4;
        self.port[port as usize].r32(offset - 4 * port)
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        let port = (offset / 4) % 4;
        self.port[port as usize].w32(offset - 4 * port, val)
    }
}

/// Standard GPIO addresses + 0x800 allow atomic port manipulation on PP502x.
///
/// Bits 8..15 of the written word define which bits are changed, bits 0..7
/// define the value of those bits.
#[derive(Debug)]
pub struct GpioBlockAtomicMirror {
    block: ArcMutexDevice<GpioBlock>,
}

impl GpioBlockAtomicMirror {
    pub fn new(block: ArcMutexDevice<GpioBlock>) -> GpioBlockAtomicMirror {
        GpioBlockAtomicMirror { block }
    }
}

impl Device for GpioBlockAtomicMirror {
    fn kind(&self) -> &'static str {
        "GPIO Port Atomic-Access Mirror"
    }

    fn probe(&self, offset: u32) -> Probe {
        Probe::from_device(&*self.block.lock().unwrap(), offset)
    }
}

impl Memory for GpioBlockAtomicMirror {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0..=0x7f => Err(InvalidAccess),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        let mut block = self.block.lock().unwrap();

        let mask = val.get_bits(8..=15) as u8;
        let val = val.get_bits(0..=7) as u8;

        match offset {
            0x0..=0x7f => {
                let old_val = block.r8(offset)?;
                block.w8(offset, (old_val & !mask) | (val & mask))?;
                Ok(())
            }
            _ => Err(Unexpected),
        }
    }
}
