pub mod fakeflash;
pub mod syscontrol;

pub use fakeflash::FakeFlash;
pub use syscontrol::SysControl;

use crate::memory::{AccessViolation, AccessViolationKind, MemResult, Memory};

/// A device which returns an AccessViolation::Unimplemented when accessed
pub struct Stub;

impl Memory for Stub {
    fn label(&self) -> String {
        "<unmapped memory>".to_string()
    }

    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        Err(AccessViolation::new(
            self.label(),
            offset,
            AccessViolationKind::Unimplemented,
        ))
    }
    fn w32(&mut self, offset: u32, _: u32) -> MemResult<()> {
        Err(AccessViolation::new(
            self.label(),
            offset,
            AccessViolationKind::Unimplemented,
        ))
    }
}
