use crate::devices::prelude::*;

use crate::signal::{self, gpio, irq};

#[derive(Debug)]
pub struct KeypadSignals<T> {
    pub action: T,
    pub up: T,
    pub down: T,
    pub left: T,
    pub right: T,
}

impl KeypadSignals<()> {
    pub fn new_tx_rx(
        notify: signal::Trigger,
    ) -> (KeypadSignals<signal::Master>, KeypadSignals<signal::Slave>) {
        let (action_tx, action_rx) = signal::new(notify.clone(), "I2C", "KeyAction");
        let (up_tx, up_rx) = signal::new(notify.clone(), "I2C", "KeyUp");
        let (down_tx, down_rx) = signal::new(notify.clone(), "I2C", "KeyDown");
        let (left_tx, left_rx) = signal::new(notify.clone(), "I2C", "KeyLeft");
        let (right_tx, right_rx) = signal::new(notify, "I2C", "KeyRight");

        (
            KeypadSignals {
                action: action_tx,
                up: up_tx,
                down: down_tx,
                left: left_tx,
                right: right_tx,
            },
            KeypadSignals {
                action: action_rx,
                up: up_rx,
                down: down_rx,
                left: left_rx,
                right: right_rx,
            },
        )
    }
}

/// I2C Controller
#[derive(Debug)]
pub struct I2CCon {
    irq: irq::Sender,
    keypad: Option<KeypadSignals<signal::Slave>>,
    hold: Option<gpio::Reciever>,

    control: u32,
    keypad_status: u32,
}

impl I2CCon {
    pub fn new(irq: irq::Sender) -> I2CCon {
        I2CCon {
            irq,
            control: 0,
            keypad_status: 0,
            keypad: None,
            hold: None,
        }
    }

    pub fn register_keypad(&mut self, keypad: KeypadSignals<signal::Slave>, hold: gpio::Reciever) {
        self.keypad = Some(keypad);
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
            0x100 => "?",
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
        match offset {
            0x000 => Err(StubRead(Warn, self.control)),
            0x004 => Err(InvalidAccess),
            0x00c => Err(StubRead(Warn, 0)),
            0x010 => Err(StubRead(Warn, 0)),
            0x014 => Err(StubRead(Warn, 0)),
            0x018 => Err(StubRead(Warn, 0)),
            0x01c => Ok(1 << 6), // never busy
            0x100 => Err(StubRead(Warn, 0)),
            0x104 => Err(StubRead(Warn, self.keypad_status | 0x0400_0000)), // never busy
            0x120 => Err(StubRead(Warn, 0)),
            0x124 => Err(StubRead(Warn, 0)),
            0x140 => {
                let (keypad, hold) = match (&self.keypad, &self.hold) {
                    (Some(keypad), Some(hold)) => (keypad, hold),
                    _ => return Err(StubRead(Warn, 0)),
                };

                let touch = None;
                let val = *0u32
                    .set_bits(0..=7, if hold.is_high() { 0x1a } else { 0 }) // 0x1a, or 0 if hold is engaged
                    .set_bit(8, keypad.action.asserted())
                    .set_bit(9, keypad.right.asserted())
                    .set_bit(10, keypad.left.asserted())
                    .set_bit(11, keypad.down.asserted())
                    .set_bit(12, keypad.up.asserted())
                    .set_bits(16..=22, touch.unwrap_or(0))
                    .set_bit(30, touch.is_some())
                    .set_bit(31, hold.is_high()); // set unless hold switch is engaged

                Err(StubRead(Warn, val))
            }
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x000 => Err(StubWrite(Warn, self.control = val)),
            0x004 => Err(StubWrite(Warn, ())),
            0x00c => Err(StubWrite(Warn, ())),
            0x010 => Err(StubWrite(Warn, ())),
            0x014 => Err(StubWrite(Warn, ())),
            0x018 => Err(StubWrite(Warn, ())),
            0x01c => Err(InvalidAccess),
            0x100 => Err(StubWrite(Warn, ())),
            0x104 => Err(StubWrite(Warn, self.keypad_status = val)),
            0x120 => Err(StubWrite(Warn, ())),
            0x124 => Err(StubWrite(Warn, ())),
            0x140 => Err(StubWrite(Warn, {
                // TODO: explore IRQ behavior if multiple I2C devices fire irqs
                self.irq.clear()
            })),
            _ => Err(Unexpected),
        }
    }
}
