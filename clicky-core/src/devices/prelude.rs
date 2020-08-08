//! The Device Prelude.
//!
//! The purpose of this module is to alleviate imports of common device traits
//! and types by adding a glob import to the top of device modules:

pub use bit_field::BitField;
pub use log::Level::*;

pub use crate::devices::{Device, Probe};
pub use crate::error::{
    MemException::{self, *},
    MemResult,
};
pub use crate::executor::*;
pub use crate::memory::Memory;
pub use crate::signal::{self, irq};
