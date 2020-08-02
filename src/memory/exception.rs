use crate::memory::MemAccess;

pub type MemResult<T> = Result<T, MemException>;

/// Context around a MemException.
#[derive(Debug, Clone)]
pub struct MemExceptionCtx {
    pub pc: u32,
    pub access: MemAccess,
    pub in_device: String,
}

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
    Fatal(String),

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
        MemException::Fatal(format!("I/O Error: {}", e))
    }
}

use crate::error::FatalError;

impl MemException {
    /// Handle the memory exception, potentially returning a FatalError.
    pub fn resolve(self, ctx: MemExceptionCtx) -> Result<(), FatalError> {
        let ctx_str = format!(
            "[pc {:#010x?}][addr {:#010x?}][{}]",
            ctx.pc, ctx.access.offset, ctx.in_device
        );

        macro_rules! mlog {
            ($($args:tt)*) => {
                log!(target: "MMIO", $($args)*)
            };
        }

        use MemException::*;
        match self {
            StubRead(level, _) => mlog!(level, "{} stubbed read ({})", ctx_str, ctx.access.val),
            StubWrite(level, ()) => mlog!(level, "{} stubbed write ({})", ctx_str, ctx.access.val),
            Log(level, msg) => mlog!(level, "{} {}", ctx_str, msg),
            // FIXME?: Misaligned access (i.e: Data Abort) should be a CPU exception
            Misaligned => {
                return Err(FatalError::MemException {
                    context: ctx,
                    reason: self,
                });
            }
            ContractViolation {
                msg,
                severity,
                stub_val,
            } => {
                // TODO: use config to determine what Error-level ContractViolation should
                // terminate execution
                if severity == log::Level::Error {
                    return Err(FatalError::MemException {
                        context: ctx,
                        reason: ContractViolation {
                            msg,
                            severity,
                            stub_val,
                        },
                    });
                } else {
                    mlog!(severity, "{} {}", ctx_str, msg)
                }
            }
            Unexpected | Unimplemented | Fatal(_) | MmuViolation | InvalidAccess => {
                return Err(FatalError::MemException {
                    context: ctx,
                    reason: self,
                })
            }
        }

        Ok(())
    }
}
