mod null;
mod raw;

pub mod backend {
    pub use super::null::Null;
    pub use super::raw::Raw;
}

/// Abstraction over different Block Device backends.
pub trait BlockDev: std::fmt::Debug {
    /// Return the length (in bytes) of the underlying medium
    fn len(&self) -> u64;
}
