use super::{Master, Slave, new as new_signal};

/// Create a new DRQ line.
pub fn new(debug_label: &'static str) -> (Sender, Receiver) {
    let (master, slave) = new_signal(None, "DRQ", debug_label);

    let sender = Sender { master };
    let receiver = Receiver { slave };

    (sender, receiver)
}

/// The receiving (DMA controller) side of a DRQ line.
#[derive(Debug, Clone)]
pub struct Receiver {
    slave: Slave,
}

impl Receiver {
    /// Checks if the peripheral is requesting a transfer.
    #[inline]
    pub fn asserted(&self) -> bool {
        self.slave.asserted()
    }
}

/// The sending (peripheral) side of a DRQ line.
#[derive(Debug, Clone)]
pub struct Sender {
    master: Master,
}

impl Sender {
    /// Set the level of the request line.
    #[inline]
    pub fn set(&mut self, requesting: bool) {
        if requesting {
            self.master.assert()
        } else {
            self.master.clear()
        }
    }

    /// Check if this sender is requesting a transfer.
    #[inline]
    pub fn is_asserting(&self) -> bool {
        self.master.is_asserting()
    }
}
