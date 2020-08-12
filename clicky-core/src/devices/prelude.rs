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

// XXX: the fact that this is required is indicative of the need to rework the
// device memory interface.
pub trait TruncateByte {
    fn trunc_to_u8(self) -> MemResult<u8>;
}

impl TruncateByte for u32 {
    fn trunc_to_u8(self) -> MemResult<u8> {
        if self > 0xff {
            Err(ContractViolation {
                msg: ">8-bit access to a 8-bit interface".into(),
                severity: Error,
                stub_val: None,
            })
        } else {
            Ok(self as u8)
        }
    }
}

impl TruncateByte for u16 {
    fn trunc_to_u8(self) -> MemResult<u8> {
        if self > 0xff {
            Err(ContractViolation {
                msg: ">8-bit access to a 8-bit interface".into(),
                severity: Error,
                stub_val: None,
            })
        } else {
            Ok(self as u8)
        }
    }
}
