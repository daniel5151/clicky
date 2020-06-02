use std::fmt::Debug;
use std::io::{self, Read, Seek, Write};

pub mod backend;
mod cfg;

pub use cfg::BlockCfg;

/// Abstraction over different Block Device backends.
#[allow(clippy::len_without_is_empty)]
pub trait BlockDev: Debug + Read + Write + Seek {
    /// Return the length (in bytes) of the underlying medium
    fn len(&self) -> io::Result<u64>;
}
