use bit_field::BitField;
use log::Level::*;

use crate::devices::util::arcmutex::ArcMutexDevice;
use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// 8-bit GPIO Port
#[derive(Debug)]
struct GpioPort {
    label: &'static str,

    enable: u8,
    output_enable: u8,
    output_val: u8,
    input_val: u8,
    interrupt_status: u8,
    interrupt_enable: u8,
    interrupt_level: u8,
}

impl GpioPort {
    fn new(label: &'static str) -> GpioPort {
        GpioPort {
            label,
            enable: 0,
            output_enable: 0,
            output_val: 0,
            input_val: 0,
            interrupt_status: 0,
            interrupt_enable: 0,
            interrupt_level: 0,
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
            0x30 => Err(StubRead(Warn, 0x00)),
            0x40 => Err(StubRead(Warn, 0x00)),
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
            0x00 => {
                self.enable = val;
                Err(StubWrite(Warn))
            }
            0x10 => {
                self.output_enable = val;
                Err(StubWrite(Warn))
            }
            0x20 => {
                self.output_val = val;
                Err(StubWrite(Warn))
            }
            0x30 => Err(InvalidAccess),
            0x40 => Err(InvalidAccess),
            0x50 => {
                self.interrupt_enable = val;
                Err(StubWrite(Warn))
            }
            0x60 => {
                self.interrupt_level = val;
                Err(StubWrite(Warn))
            }
            0x70 => Err(StubWrite(Warn)),
            _ => Err(Unexpected),
        }
    }
}

/// Block of 4 GPIO ports on the PP5020.
#[derive(Debug)]
pub struct GpioBlock {
    port: [GpioPort; 4],
}

impl GpioBlock {
    pub fn new(labels: [&'static str; 4]) -> GpioBlock {
        GpioBlock {
            port: [
                GpioPort::new(labels[0]),
                GpioPort::new(labels[1]),
                GpioPort::new(labels[2]),
                GpioPort::new(labels[3]),
            ],
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
        let block = self.block.lock().unwrap();
        // XXX: There doesn't seem to be a good way to implement this using the current
        // probe system...
        Probe::from_device(&*block, offset)
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
                // XXX: the whole point is to do a _side effect free_ read + modify of the GPIO
                // ports. Using r8 here technically breaks this invariant, though at the moment,
                // this works fine (AFAIK GPIO reads are side-effect free). This might lead to a
                // subtle bug if GPIO reads result in side-effects though...
                let old_val = block.r8(offset)?;
                block.w8(offset, (old_val & !mask) | (val & mask))?;
                Ok(())
            }
            _ => Err(Unexpected),
        }
    }
}
