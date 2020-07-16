use crate::memory::{MemException, MemExceptionCtx};

/// An error type from which the system cannot recover.
#[derive(Debug, Clone)]
pub enum FatalError {
    /// An unrecoverable memory exception.
    MemException {
        context: MemExceptionCtx,
        reason: MemException,
    },
}
