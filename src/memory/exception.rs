pub type MemResult<T> = Result<T, MemException>;

/// Exception resulting from a memory access.
#[derive(Debug, Clone)]
pub enum MemException {
    // -- Non-Fatal Errors -- //
    /// Memory location is using a stubbed read implementation.
    StubRead(log::Level, u32),
    /// Memory location is using a stubbed write implementation.
    StubWrite(log::Level, ()),
    /// Success, but also log log a message.
    // HACK: there should be some way to pipe context to the devices themselves?
    Log(log::Level, String),

    // -- Internal Emulator Errors -- //
    /// Memory location that shouldn't have been accessed.
    Unexpected,
    /// Memory location hasn't been implemented.
    Unimplemented,
    /// An unrecoverable error which should immediately terminate execution.
    FatalError(String),

    // -- Guest Access Violations -- //
    /// Attempted to access a device at an invalid offset.
    Misaligned,
    /// Attempted to read a write-only register / write to a read-only register.
    InvalidAccess,
    /// Invalid access in a protected memory region.
    MmuViolation,
    /// Performed an unexpected action on the device.
    ///
    /// e.g: sending an invalid command byte to an IDE device, improper
    /// sequencing when writing configuration data, etc...
    ContractViolation {
        msg: String,
        severity: log::Level,
        stub_val: Option<u32>,
    },
}

// Maybe this should be a separate error type at some point, but this is fine
// for now.
impl From<std::io::Error> for MemException {
    fn from(e: std::io::Error) -> MemException {
        MemException::FatalError(format!("I/O Error: {}", e))
    }
}
