//! Block device interface and backend implementations.

use std::fmt::Debug;

use futures::io::{AsyncRead, AsyncSeek, AsyncWrite};

pub mod backend;
mod cfg;

pub use cfg::BlockCfg;

/// Abstraction over different Block Device backends.
#[allow(clippy::len_without_is_empty)]
pub trait BlockDev: Unpin + Debug + AsyncRead + AsyncSeek + AsyncWrite {
    /// Return the length (in bytes) of the underlying medium.
    fn len(&self) -> u64;
}
