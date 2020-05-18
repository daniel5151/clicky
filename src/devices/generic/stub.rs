use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// Generic stub device. Reads/Writes result in StubRead/StubWrite MemResults.
/// THIS DEVICE SHOULD BE USED SPARINGLY!
#[derive(Debug)]
pub struct Stub {
    label: String,
}

impl Stub {
    pub fn new(label: String) -> Stub {
        Stub { label }
    }
}

impl Device for Stub {
    fn kind(&self) -> &'static str {
        "Stub"
    }

    fn label(&self) -> Option<&str> {
        Some(&self.label)
    }

    fn probe(&self, _offset: u32) -> Probe<'_> {
        Probe::Register("<stub>")
    }
}

impl Memory for Stub {
    fn r32(&mut self, _offset: u32) -> MemResult<u32> {
        Err(StubRead(0x00))
    }

    fn w32(&mut self, _offset: u32, _val: u32) -> MemResult<()> {
        Err(StubWrite)
    }
}
