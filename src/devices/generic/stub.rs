use log::Level::*;

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// Generic stub device. Reads/Writes result in Error-level StubRead/StubWrites.
/// THIS DEVICE SHOULD BE USED SPARINGLY AS A WAY TO MAKE FORWARD PROGRESS!
#[derive(Debug)]
pub struct Stub {
    label: &'static str,
}

impl Stub {
    pub fn new(label: &'static str) -> Stub {
        Stub { label }
    }
}

impl Device for Stub {
    fn kind(&self) -> &'static str {
        "Stub"
    }

    fn label(&self) -> Option<&'static str> {
        Some(&self.label)
    }

    fn probe(&self, _offset: u32) -> Probe {
        Probe::Register("<stub>")
    }
}

impl Memory for Stub {
    fn r32(&mut self, _offset: u32) -> MemResult<u32> {
        Err(StubRead(Error, 0))
    }

    fn w32(&mut self, _offset: u32, _val: u32) -> MemResult<()> {
        Err(StubWrite(Error))
    }
}
