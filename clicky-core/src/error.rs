use crate::memory::MemAccess;

pub type MemResult<T> = Result<T, MemException>;
pub type FatalMemResult<T> = Result<T, FatalMemException>;

/// Exception resulting from a memory access.
#[derive(Debug, Clone)]
pub enum MemException {
    // -- Non-Fatal Errors -- //
    /// Memory location is using a stubbed read implementation.
    StubRead(log::Level, u32),
    /// Memory location is using a stubbed write implementation.
    StubWrite(log::Level, ()),

    // -- Internal Emulator Errors -- //
    /// Memory location that shouldn't have been accessed.
    Unexpected,
    /// Memory location hasn't been implemented.
    Unimplemented,
    /// An unrecoverable error which should immediately terminate execution.
    Fatal(String),

    // -- Wrappers -- //
    /// Denotes exception as having occurred during an i2c operation.
    ///
    /// XXX: this is a really ugly approach to handling i2c exceptions...
    I2CException {
        e: Box<MemException>,
        // context
        access: MemAccess,
        in_device: String,
    },

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

/// Context around a MemException.
#[derive(Debug, Clone)]
pub struct MemExceptionCtx {
    pub pc: u32,
    pub access: MemAccess,
    pub in_device: String,
}

impl std::fmt::Display for MemExceptionCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[pc {:#010x?}][addr {:#010x?}][{}]",
            self.pc, self.access.offset, self.in_device
        )
    }
}

/// An unrecoverable memory exception.
#[derive(Debug, Clone)]
pub struct FatalMemException {
    context: MemExceptionCtx,
    reason: MemException,
}

impl MemException {
    /// Handle the memory exception, potentially returning a FatalMemException.
    pub fn resolve(
        self,
        target: &'static str,
        ctx: MemExceptionCtx,
    ) -> Result<(), FatalMemException> {
        macro_rules! mlog {
            (($level:ident, $ctx:ident) => ($($args:tt)*)) => {
                if log_enabled!($level) {
                    let $ctx = $ctx;
                    log!(target: target, $level, $($args)*)
                }
            };
        }

        use MemException::*;
        match self {
            StubRead(level, _) => {
                mlog! { (level, ctx) => ("{} stubbed read ({})", ctx, ctx.access.val) }
            }
            StubWrite(level, ()) => {
                mlog! { (level, ctx) => ("{} stubbed write ({})", ctx, ctx.access.val) }
            }
            // XXX: absolutely disgusting way to handle i2c exceptions, yikes
            I2CException {
                e,
                access,
                in_device,
            } => e.resolve(
                "I2C",
                MemExceptionCtx {
                    pc: ctx.pc,
                    access,
                    in_device,
                },
            )?,
            // FIXME?: Misaligned access (i.e: Data Abort) should be a CPU exception
            Misaligned => {
                return Err(FatalMemException {
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
                    return Err(FatalMemException {
                        context: ctx,
                        reason: ContractViolation {
                            msg,
                            severity,
                            stub_val,
                        },
                    });
                } else {
                    mlog! { (severity, ctx) => ("{} {}", ctx, msg) }
                }
            }
            Unexpected | Unimplemented | Fatal(_) | MmuViolation | InvalidAccess => {
                return Err(FatalMemException {
                    context: ctx,
                    reason: self,
                })
            }
        }

        Ok(())
    }
}
