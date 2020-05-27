use std::fmt::Debug;
use std::io::{self, Read, Seek, Write};

mod null;
mod raw;

pub mod backend {
    pub use super::null::Null;
    pub use super::raw::Raw;
}

/// Abstraction over different Block Device backends.
pub trait BlockDev: Debug + Read + Write + Seek {
    /// Return the length (in bytes) of the underlying medium
    fn len(&self) -> io::Result<u64>;
}
