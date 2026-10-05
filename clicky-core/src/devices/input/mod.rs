//! Input-related devices.

use crate::devices::prelude::*;

use std::sync::{Arc, Mutex};

use crate::signal;

pub mod clickwheel;

pub trait OptoDevice: std::fmt::Debug + Send + Sync {
    fn read_status(&mut self, command: Option<u32>) -> MemResult<u32>;
}

#[derive(Debug, Clone)]
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
