mod raw;

pub mod kind {
    pub use super::raw::RawBlockDev;
}

/// Abstraction over different Block Device backends.
#[derive(Debug)]
pub enum BlockDev {
    /// Reads return zeros, Writes are ignored. Infinite size.
    Null,
    /// Raw, file-backed block device. No fancy features, just raw 1:1 access to
    /// the underlying file's contents.
    Raw(kind::RawBlockDev),
}
