use crate::devices::prelude::*;

/// Generic stub device. Reads/Writes result in Error-level StubRead/StubWrites.
///
/// THIS DEVICE SHOULD BE USED SPARINGLY AS A DEVELOPMENT AID! Please create
/// _concrete_ devices when possible.
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
        Some(self.label)
    }

    fn probe(&self, _offset: u32) -> Probe {
        Probe::Unmapped
    }
}

impl Memory for Stub {
    fn r8(&mut self, _offset: u32) -> MemResult<u8> {
        Err(StubRead(Error, 0))
    }

    fn r16(&mut self, _offset: u32) -> MemResult<u16> {
        Err(StubRead(Error, 0))
    }

    fn r32(&mut self, _offset: u32) -> MemResult<u32> {
        Err(StubRead(Error, 0))
    }

    fn w8(&mut self, _offset: u32, _val: u8) -> MemResult<()> {
        Err(StubWrite(Error, ()))
    }

    fn w16(&mut self, _offset: u32, _val: u16) -> MemResult<()> {
        Err(StubWrite(Error, ()))
    }

    fn w32(&mut self, _offset: u32, _val: u32) -> MemResult<()> {
        Err(StubWrite(Error, ()))
    }
}
