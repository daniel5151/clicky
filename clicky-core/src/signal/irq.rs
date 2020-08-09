//! IRQ signaling and notification.

use super::{new as new_signal, Master, Slave, Trigger, TriggerKind};

/// Create a new IRQ line. Updates `notify` when the sender asserts the IRQ.
pub fn new(notify: Pending, debug_label: &'static str) -> (Sender, Reciever) {
    let (master, slave) = new_signal(notify.trigger, "IRQ", debug_label);

    let sender = Sender { master };
    let reciever = Reciever { slave };

    (sender, reciever)
}

/// Tracks IRQ assertions across one-or-more IRQ lines.
#[derive(Debug, Clone)]
pub struct Pending {
    trigger: Trigger,
}

impl Default for Pending {
    fn default() -> Pending {
        Pending::new()
    }
}

impl Pending {
    pub fn new() -> Pending {
        Pending {
            trigger: Trigger::new(TriggerKind::Hi),
        }
    }

    /// Checks if any connected IRQs have been fired.
    #[inline]
    pub fn check(&self) -> bool {
        self.trigger.check()
    }

    /// Checks if any connected IRQs have been fired since the last call to
    /// `check_pending`.
    #[inline]
    pub fn clear(&self) -> bool {
        self.trigger.check()
    }
}

/// The receiving side of an IRQ line.
#[derive(Debug, Clone)]
pub struct Reciever {
    slave: Slave,
}

impl Reciever {
    /// Checks if the IRQ has been set.
    #[inline]
    pub fn asserted(&self) -> bool {
        self.slave.asserted()
    }
}

/// The sending side of an IRQ line. Senders can be cloned, whereupon each
/// Sender will share the signal line. The signal is asserted if ANY Sender
/// asserts, and cleared only if ALL Senders have called clear.
#[derive(Debug, Clone)]
pub struct Sender {
    master: Master,
}

impl Sender {
    /// Signal an IRQ.
    #[inline]
    pub fn assert(&mut self) {
        self.master.assert()
    }

    /// Clears an IRQ.
    #[inline]
    pub fn clear(&mut self) {
        self.master.clear()
    }

    /// Check if this sender is setting the signal high.
    #[inline]
    pub fn is_asserting(&self) -> bool {
        self.master.is_asserting()
    }
}
