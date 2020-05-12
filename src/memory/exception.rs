pub type MemResult<T> = Result<T, MemException>;

/// Denotes some sort of exception stemming from a memory access.
#[derive(Debug, Clone)]
pub enum MemException {
    // -- Internal Emulator Errors -- //
    /// Memory location that shouldn't have been accessed
    Unexpected,
    /// Memory location hasn't been implemented
    Unimplemented,
    /// Memory location is using a stubbed read implementation
    StubRead(u32),
    /// Memory location is using a stubbed write implementation
    StubWrite,

    // -- Guest Access Violations -- //
    /// Attempted to access a device at an invalid offset
    Misaligned,
    /// Attempted to read a write-only register / write to a read-only register
    InvalidAccess,
    /// Device Contract Violation
    ContractViolation {
        msg: String,
        severity: log::Level,
        stub_val: Option<u32>,
    },
}
