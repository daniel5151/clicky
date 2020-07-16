//! Block device backends.

mod mem;
mod null;
mod raw;

pub use mem::Mem;
pub use null::Null;
pub use raw::Raw;
