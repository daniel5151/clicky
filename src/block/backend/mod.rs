//! Block device backends.

mod null;
mod raw;

pub use null::Null;
pub use raw::Raw;
