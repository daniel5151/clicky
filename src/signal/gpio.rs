//! GPIO signaling and notification.

use super::{new as new_signal, Master, Slave, Trigger, TriggerKind};

/// Create a new GPIO line. Updates `notify` whenever the sender updates the
/// signal.
pub fn new(notify: Changed, debug_label: &'static str) -> (Sender, Reciever) {
    let (master, slave) = new_signal(notify.trigger, "GPIO", debug_label);

    let sender = Sender { master };
    let reciever = Reciever { slave };

    (sender, reciever)
}

/// Tracks GPIO signal changes across one-or-more GPIO lines.
#[derive(Debug, Clone)]
pub struct Changed {
    trigger: Trigger,
}

impl Default for Changed {
    fn default() -> Changed {
        Changed::new()
    }
}

impl Changed {
    pub fn new() -> Changed {
        Changed {
            trigger: Trigger::new(TriggerKind::Edge),
        }
    }

    /// Checks if any connected GPIO lines have changed since the last call to
    /// `check_and_clear`.
    pub fn check_and_clear(&self) -> bool {
        self.trigger.check_and_clear()
    }
}

/// The receiving side of a GPIO line.
#[derive(Debug, Clone)]
pub struct Reciever {
    slave: Slave,
}

impl Reciever {
    /// Checks if the GPIO line is high.
    pub fn is_high(&self) -> bool {
        self.slave.asserted()
    }
}

/// The sending side of a GPIO line. Senders can be cloned, whereupon each
/// Sender will share the signal line. The signal is asserted if ANY Sender
/// asserts, and cleared only if ALL Senders have called clear.
#[derive(Debug)]
pub struct Sender {
    master: Master,
}

impl Sender {
    /// Set the GPIO line high.
    pub fn set_high(&mut self) {
        self.master.assert()
    }

    /// Set the GPIO line low.
    pub fn set_low(&mut self) {
        self.master.clear()
    }

    /// Check if this sender is setting the signal high.
    pub fn is_set_high(&self) -> bool {
        self.master.is_asserting()
    }
}
