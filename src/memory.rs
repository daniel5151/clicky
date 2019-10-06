#[derive(Debug, Copy, Clone)]
pub enum AccessViolationKind {
    Misaligned,
    Unimplemented,
}

/// Denotes some sort of memory access faliure
#[derive(Debug)]
pub struct AccessViolation {
    label: String,
    addr: u32,
    kind: AccessViolationKind,
}

impl AccessViolation {
    pub fn new(label: String, addr: u32, kind: AccessViolationKind) -> AccessViolation {
        AccessViolation { label, addr, kind }
    }

    pub fn addr(&self) -> u32 {
        self.addr
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn kind(&self) -> AccessViolationKind {
        self.kind
    }
}

pub type MemResult<T> = Result<T, AccessViolation>;

/// Utility methods to make working with MemResults more ergonomic
pub trait MemResultExt {
    /// If the MemResult is an error, add `offset` to the underlying addr, and
    /// prefix `label` to the address
    fn map_memerr_ctx(self, offset: u32, label: String) -> Self;
    /// If the MemResult is an error, add `offset` to the underlying addr
    fn map_memerr_offset(self, offset: u32) -> Self;
}

impl<T> MemResultExt for MemResult<T> {
    fn map_memerr_offset(self, offset: u32) -> Self {
        self.map_err(|mut violation| {
            violation.addr += offset;
            violation
        })
    }

    fn map_memerr_ctx(self, offset: u32, label: String) -> Self {
        self.map_err(|mut violation| {
            violation.label = label + ":" + &violation.label;
            violation.addr += offset;
            violation
        })
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! unimplemented_offset {
    () => {
        Err(crate::memory::AccessViolation::new(
            "<unimplemented offset>".to_string(),
            0,
            crate::memory::AccessViolationKind::Unimplemented,
        ))
    };
}

/// Common memory trait used throughout Clicky.
/// Default implementations for 8-bit and 16-bit read/write is to return a
/// [AccessViolation::Misaligned]
pub trait Memory {
    fn label(&self) -> String;

    fn r32(&mut self, offset: u32) -> MemResult<u32>;
    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()>;

    fn r8(&mut self, offset: u32) -> MemResult<u8> {
        if offset & 0x3 != 0 {
            Err(crate::memory::AccessViolation::new(
                self.label(),
                offset,
                crate::memory::AccessViolationKind::Misaligned,
            ))
        } else {
            Memory::r32(self, offset).map(|v| v as u8)
        }
    }
    fn r16(&mut self, offset: u32) -> MemResult<u16> {
        if offset & 0x3 != 0 {
            Err(crate::memory::AccessViolation::new(
                self.label(),
                offset,
                crate::memory::AccessViolationKind::Misaligned,
            ))
        } else {
            Memory::r32(self, offset).map(|v| v as u16)
        }
    }
    fn w8(&mut self, offset: u32, val: u8) -> MemResult<()> {
        if offset & 0x3 != 0 {
            Err(crate::memory::AccessViolation::new(
                self.label(),
                offset,
                crate::memory::AccessViolationKind::Misaligned,
            ))
        } else {
            Memory::w32(self, offset, val as u32)
        }
    }
    fn w16(&mut self, offset: u32, val: u16) -> MemResult<()> {
        if offset & 0x3 != 0 {
            Err(crate::memory::AccessViolation::new(
                self.label(),
                offset,
                crate::memory::AccessViolationKind::Misaligned,
            ))
        } else {
            Memory::w32(self, offset, val as u32)
        }
    }
}

impl Memory for Box<dyn Memory> {
    fn label(&self) -> String {
        (**self).label()
    }

    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        (**self).r32(offset)
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        (**self).w32(offset, val)
    }

    fn r8(&mut self, offset: u32) -> MemResult<u8> {
        (**self).r8(offset)
    }
    fn r16(&mut self, offset: u32) -> MemResult<u16> {
        (**self).r16(offset)
    }
    fn w8(&mut self, offset: u32, val: u8) -> MemResult<()> {
        (**self).w8(offset, val)
    }
    fn w16(&mut self, offset: u32, val: u16) -> MemResult<()> {
        (**self).w16(offset, val)
    }
}
