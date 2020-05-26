use super::BlockDev;

use std::fs::File;

/// Raw, file-backed block device. No fancy features, just raw 1:1 access to
/// the underlying file's contents.
#[derive(Debug)]
pub struct Raw {
    file: File,
}

impl Raw {
    pub fn new(file: File) -> Raw {
        Raw { file }
    }
}

impl BlockDev for Raw {
    fn len(&self) -> u64 {
        unimplemented!()
    }
}
