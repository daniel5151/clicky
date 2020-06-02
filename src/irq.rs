//! IRQ signaling and notification.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// TODO: Explore using a less-restrictive ordering. `Ordering::SeqCst` was
// picked just to "play it safe"

/// Create a new IRQ line. Updates `notify` whenever the the sender
/// asserts/clears the IRQ.
pub fn new(notify: Pending, debug_label: &'static str) -> (Sender, Reciever) {
    let signal = Arc::new(AtomicBool::new(false));

    let sender = Sender {
        pending: notify.signal,
        signal: Arc::clone(&signal),
        debug_label,
    };
    let reciever = Reciever { signal };

    (sender, reciever)
}

/// Check if _any_ connected IRQs have been triggered.
#[derive(Debug, Default, Clone)]
pub struct Pending {
    signal: Arc<AtomicBool>,
}

impl Pending {
    pub fn new() -> Pending {
        Pending {
            signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Checks if the IRQ has been set.
    pub fn has_pending(&self) -> bool {
        self.signal.load(Ordering::SeqCst)
    }
}

/// The receiving side of an IRQ line.
///
/// Typically passed to an IRQ controller and checked during IRQ resolution.
#[derive(Debug, Clone)]
pub struct Reciever {
    signal: Arc<AtomicBool>,
}

impl Reciever {
    /// Checks if the IRQ has been set.
    pub fn is_set(&self) -> bool {
        self.signal.load(Ordering::SeqCst)
    }
}

/// The sending side of an IRQ line.
///
/// Typically passed to Devices.
#[derive(Debug, Clone)]
pub struct Sender {
    pending: Arc<AtomicBool>,
    signal: Arc<AtomicBool>,
    debug_label: &'static str,
}

impl Sender {
    /// Signal an IRQ.
    pub fn assert(&self) {
        if log_enabled!(target: "IRQ", log::Level::Trace) {
            trace!(target: "IRQ", "Asserted IRQ: {}", self.debug_label);
        }

        self.pending.store(true, Ordering::SeqCst);
        self.signal.store(true, Ordering::SeqCst);
    }

    /// Clears an IRQ.
    pub fn clear(&self) {
        if log_enabled!(target: "IRQ", log::Level::Trace) {
            trace!(target: "IRQ", "Cleared IRQ: {}", self.debug_label);
        }

        self.pending.store(false, Ordering::SeqCst);
        self.signal.store(false, Ordering::SeqCst);
    }
}
