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
        let (action_tx, action_rx) = signal::new(notify.clone(), "I2C", "KeyAction");
        let (up_tx, up_rx) = signal::new(notify.clone(), "I2C", "KeyUp");
        let (down_tx, down_rx) = signal::new(notify.clone(), "I2C", "KeyDown");
        let (left_tx, left_rx) = signal::new(notify.clone(), "I2C", "KeyLeft");
        let (right_tx, right_rx) = signal::new(notify.clone(), "I2C", "KeyRight");
        let (wheel_tx, wheel_rx) = signal::new(notify, "I2C", "Wheel");

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
pub struct I2CCon {
    irq: irq::Sender,
    controls: Option<Controls<signal::Slave>>,
    hold: Option<gpio::Reciever>,

    busy: bool,
    control: u32,
    controls_status: u32,
}

impl I2CCon {
    pub fn new(irq: irq::Sender) -> I2CCon {
        I2CCon {
            irq,
            controls: None,
            hold: None,

            busy: false,
            control: 0,
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

impl Device for I2CCon {
    fn kind(&self) -> &'static str {
        "I2CCon"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x000 => "Control",
            0x004 => "Addr",
            0x00c => "Data0",
            0x010 => "Data1",
            0x014 => "Data2",
            0x018 => "Data3",
            0x01c => "Status",
            0x100 => "(?) Keypad IRQ clear",
            0x104 => "(?) Keypad Status",
            0x120 => "?",
            0x124 => "?",
            0x140 => "Scroll Wheel + Keypad",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for I2CCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        let ret = match offset {
            0x000 => Err(StubRead(Warn, self.control)),
            0x004 => Err(InvalidAccess),
            0x00c => Err(StubRead(Warn, 0)),
            0x010 => Err(StubRead(Warn, 0)),
            0x014 => Err(StubRead(Warn, 0)),
            0x018 => Err(StubRead(Warn, 0)),
            0x01c => {
                // jiggle the busy status bit
                self.busy = !self.busy;
                Ok((self.busy as u32) << 6)
            }
            0x100 => Err(StubRead(Warn, 0)),
            0x104 => Err(StubRead(Warn, self.controls_status | 0x0400_0000)), // never busy
            0x120 => Err(StubRead(Warn, 0)),
            0x124 => Err(StubRead(Warn, 0)),
            0x140 => {
                let (controls, hold) = match (&self.controls, &self.hold) {
                    (Some(controls), Some(hold)) => (controls, hold),
                    _ => return Err(StubRead(Warn, 0)),
                };

                let val = *0u32
                    .set_bits(0..=7, if hold.is_high() { 0x1a } else { 0 }) // 0x1a, or 0 if hold is engaged
                    .set_bit(8, controls.action.asserted())
                    .set_bit(9, controls.right.asserted())
                    .set_bit(10, controls.left.asserted())
                    .set_bit(11, controls.down.asserted())
                    .set_bit(12, controls.up.asserted())
                    .set_bits(16..=22, *controls.wheel.1.lock().unwrap() as u32)
                    .set_bit(30, true)
                    // .set_bit(30, controls.wheel.0.asserted())
                    .set_bit(31, hold.is_high()); // set unless hold switch is engaged

                Err(StubRead(Warn, val))
            }
            _ => Err(Unexpected),
        };

        match ret {
            Ok(v) => Ok(v),
            Err(StubRead(_, v)) => Ok(v),
            Err(_) => ret,
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        let ret = match offset {
            0x000 => Err(StubWrite(Warn, self.control = val)),
            0x004 => Err(StubWrite(Warn, ())),
            0x00c => Err(StubWrite(Warn, ())),
            0x010 => Err(StubWrite(Warn, ())),
            0x014 => Err(StubWrite(Warn, ())),
            0x018 => Err(StubWrite(Warn, ())),
            0x01c => Err(InvalidAccess),
            0x100 => Err(StubWrite(Warn, {
                // TODO: cross-reference this with other software (not just Rockbox)
                self.irq.clear()
            })),
            0x104 => Err(StubWrite(Warn, self.controls_status = val)),
            0x120 => Err(StubWrite(Warn, ())),
            0x124 => Err(StubWrite(Warn, ())),
            0x140 => Err(StubWrite(Warn, {
                // TODO: explore IRQ behavior if multiple I2C devices fire irqs
                self.irq.clear()
            })),
            _ => Err(Unexpected),
        };

        match ret {
            Ok(()) => Ok(()),
            Err(StubWrite(_, _)) => Ok(()),
            Err(_) => ret,
        }
    }
}
