use super::BlockDev;

/// Null block device.
#[derive(Debug)]
pub struct Null {
    len: u64,
}

impl Null {
    pub fn new(reported_len: u64) -> Null {
        Null { len: reported_len }
    }
}

impl BlockDev for Null {
    fn len(&self) -> u64 {
        self.len
    }
}
