//! The Device Prelude.
//!
//! The purpose of this module is to alleviate imports of common device traits
//! and types by adding a glob import to the top of device modules:

pub use bit_field::BitField;
pub use log::Level::*;

pub use crate::devices::{Device, Probe};
pub use crate::memory::{
    MemException::{self, *},
    MemResult, Memory,
};
