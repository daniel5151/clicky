use crate::devices::prelude::*;

use std::sync::{Arc, Mutex};

use crate::signal::{self, gpio};

#[derive(Debug)]
pub struct Controls<T> {
    pub action: T,
    pub up: T,
    pub down: T,
    pub left: T,
    pub right: T,
    pub wheel: (T, Arc<Mutex<u8>>),
}

impl Controls<()> {
    pub fn new_tx_rx(
        notify: signal::Trigger,
    ) -> (Controls<signal::Master>, Controls<signal::Slave>) {
        let (action_tx, action_rx) = signal::new(notify.clone(), "Controls", "KeyAction");
        let (up_tx, up_rx) = signal::new(notify.clone(), "Controls", "KeyUp");
        let (down_tx, down_rx) = signal::new(notify.clone(), "Controls", "KeyDown");
        let (left_tx, left_rx) = signal::new(notify.clone(), "Controls", "KeyLeft");
        let (right_tx, right_rx) = signal::new(notify.clone(), "Controls", "KeyRight");
        let (wheel_tx, wheel_rx) = signal::new(notify, "Controls", "Wheel");

        let wheel_data = Arc::new(Mutex::new(0));

        (
            Controls {
                action: action_tx,
                up: up_tx,
                down: down_tx,
                left: left_tx,
                right: right_tx,
                wheel: (wheel_tx, wheel_data.clone()),
            },
            Controls {
                action: action_rx,
                up: up_rx,
                down: down_rx,
                left: left_rx,
                right: right_rx,
                wheel: (wheel_rx, wheel_data),
            },
        )
    }
}

/// I2C Controller
#[derive(Debug)]
pub struct OptoWheel {
    irq: irq::Sender,
    controls: Option<Controls<signal::Slave>>,
    hold: Option<gpio::Reciever>,

    controls_status: u32,
}

impl OptoWheel {
    pub fn new(irq: irq::Sender) -> OptoWheel {
        OptoWheel {
            irq,
            controls: None,
            hold: None,

            controls_status: 0,
        }
    }

    pub fn register_controls(&mut self, controls: Controls<signal::Slave>, hold: gpio::Reciever) {
        self.controls = Some(controls);
        self.hold = Some(hold);
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
            0x20 => "?",
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
                let (controls, hold) = match (&self.controls, &self.hold) {
                    (Some(controls), Some(hold)) => (controls, hold),
                    _ => return Err(Fatal("no controls registered with i2c".into())),
                };

                let val = *0u32
                    .set_bits(0..=7, if hold.is_high() { 0x1a } else { 0 }) // 0x1a, or 0 if hold is engaged
                    .set_bit(8, controls.action.asserted())
                    .set_bit(9, controls.right.asserted())
                    .set_bit(10, controls.left.asserted())
                    .set_bit(11, controls.down.asserted())
                    .set_bit(12, controls.up.asserted())
                    .set_bits(16..=22, *controls.wheel.1.lock().unwrap() as u32)
                    .set_bit(30, true) // FIXME: don't always return clickwheel active?
                    .set_bit(31, hold.is_high()); // set unless hold switch is engaged

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
            0x20 => Err(StubWrite(Debug, ())),
            0x24 => Err(StubWrite(Debug, ())),
            0x40 => Err(StubWrite(Debug, {
                // TODO: explore IRQ behavior if multiple I2C devices fire irqs
                self.irq.clear()
            })),
            _ => Err(Unexpected),
        }
    }
}
