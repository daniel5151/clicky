//! General signaling and notification mechanism. Used to implement GPIO, IRQs,
//! etc...

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::Arc;

pub mod gpio;
pub mod irq;

// TODO: Explore using a less-restrictive ordering. `Ordering::SeqCst` was
// picked just to "play it safe"

/// Create a new signal.
pub fn new(
    trigger: Trigger,
    debug_group: &'static str,
    debug_label: &'static str,
) -> (Master, Slave) {
    let signal = Arc::new(AtomicIsize::new(0));

    let sender = Master {
        trigger,
        own_signal: false,
        signal: Arc::clone(&signal),
        debug_group,
        debug_label,
    };
    let reciever = Slave { signal };

    (sender, reciever)
}

/// Determines a `Trigger`'s behavior.
#[derive(Debug, Copy, Clone)]
pub enum TriggerKind {
    /// Triggered when the signal goes high.
    Hi,
    /// Triggered when the signal goes low.
    Lo,
    /// Triggered when the signal changes.
    Edge,
}

/// A way to hook into (one or more) signals and get notified of any changes.
#[derive(Debug, Clone)]
pub struct Trigger {
    kind: TriggerKind,
    trigger: Arc<AtomicBool>,
}

impl Trigger {
    fn update(&self, old_val: bool, new_val: bool) {
        use TriggerKind::*;
        match self.kind {
            Hi => {
                if !old_val && new_val {
                    self.trigger.store(true, Ordering::SeqCst)
                }
            }
            Lo => {
                if old_val && !new_val {
                    self.trigger.store(true, Ordering::SeqCst)
                }
            }
            Edge => {
                if (old_val && !new_val) || (!old_val && new_val) {
                    self.trigger.store(true, Ordering::SeqCst)
                }
            }
        }
    }

    /// Create a new `Trigger`.
    pub fn new(kind: TriggerKind) -> Trigger {
        Trigger {
            kind,
            trigger: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Retrieves and un-sets the trigger.
    pub fn check_and_clear(&self) -> bool {
        self.trigger.fetch_and(false, Ordering::SeqCst)
    }
}

/// The receiving side of a signal line. Able to query the signal level, but not
/// change it.
#[derive(Debug, Clone)]
pub struct Slave {
    signal: Arc<AtomicIsize>,
}

impl Slave {
    /// Checks if the signal is high.
    pub fn asserted(&self) -> bool {
        self.signal.load(Ordering::SeqCst) != 0
    }
}

/// The sending side of a signal line. Able to assert/clear the signal level,
/// but not read it. Masters can be cloned, whereupon each Master will share the
/// signal line. The signal is asserted if ANY Master asserts, and cleared only
/// if ALL Masters have called clear.
#[derive(Debug, Clone)]
pub struct Master {
    trigger: Trigger,
    own_signal: bool,
    signal: Arc<AtomicIsize>,
    debug_group: &'static str,
    debug_label: &'static str,
}

impl Master {
    /// Set the signal high.
    pub fn assert(&mut self) {
        if self.own_signal {
            return;
        }

        if log_enabled!(target: self.debug_group, log::Level::Trace) {
            trace!(target: self.debug_group, "Asserted {}:{}", self.debug_group, self.debug_label);
        }

        self.own_signal = true;
        let old_val = self.signal.fetch_add(1, Ordering::SeqCst);
        assert!(old_val >= 0);
        self.trigger.update(old_val != 0, true);
    }

    /// Set the signal low.
    pub fn clear(&mut self) {
        if !self.own_signal {
            return;
        }

        if log_enabled!(target: self.debug_group, log::Level::Trace) {
            trace!(target: self.debug_group, "Cleared {}:{}", self.debug_group, self.debug_label);
        }

        self.own_signal = false;
        let old_val = self.signal.fetch_sub(1, Ordering::SeqCst);
        assert!(old_val > 0);
        self.trigger.update(old_val != 0, false);
    }

    /// Check if this Master is asserting the signal.
    pub fn is_asserting(&self) -> bool {
        self.own_signal
    }
}
