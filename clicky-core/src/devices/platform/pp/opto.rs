use crate::devices::prelude::*;

use crate::devices::input::OptoDevice;

/// I2C Controller
#[derive(Debug)]
pub struct OptoWheel {
    irq: irq::Sender,
    device: Option<Box<dyn OptoDevice>>,

    controls_status: u32,
    pending_cmd: Option<u32>,
}

impl OptoWheel {
    pub fn new(irq: irq::Sender) -> OptoWheel {
        OptoWheel {
            irq,
            device: None,

            controls_status: 0,
            pending_cmd: None,
        }
    }

    pub fn attach(&mut self, device: Box<dyn OptoDevice>) {
        self.device = Some(device);
    }

    fn device(&mut self) -> MemResult<&mut Box<dyn OptoDevice>> {
        self.device
            .as_mut()
            .ok_or_else(|| Fatal("no device attached to the opto controller".into()))
    }

    pub fn on_change(&mut self) {
        self.irq.assert()
    }
}

impl Device for OptoWheel {
    fn kind(&self) -> &'static str {
        "OptoWheel"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "(?) Keypad IRQ clear",
            0x04 => "(?) Keypad Status",
            0x20 => "(?) Explicit command",
            0x24 => "?",
            0x40 => "Scroll Wheel + Keypad",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for OptoWheel {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => Err(StubRead(Debug, 0)),
            0x04 => Err(StubRead(Debug, self.controls_status | 0x0400_0000)), // never busy
            0x20 => Err(Unimplemented),
            0x24 => Err(Unimplemented),
            0x40 => {
                let command = self.pending_cmd.take();

                let val = self.device()?.read_status(command)?;

                Err(StubRead(Debug, val))
            }
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => Err(StubWrite(Debug, {
                // TODO: cross-reference this with other software (not just Rockbox)
                self.irq.clear()
            })),
            0x04 => Err(StubWrite(Debug, self.controls_status = val)),
            0x20 => Err(StubWrite(Debug, {
                if val.get_bit(31) {
                    self.pending_cmd = Some(val);
                    self.irq.assert();
                }
            })),
            0x24 => Err(StubWrite(Debug, ())),
            0x40 => Err(StubWrite(Debug, {
                // the handler acks by writing here, which also retires the
                // outstanding command reply
                self.pending_cmd = None;
                // TODO: explore IRQ behavior if multiple I2C devices fire irqs
                self.irq.clear()
            })),
            _ => Err(Unexpected),
        }
    }
}
