use log::*;

use crate::devices::{Device, Probe};
use crate::memory::{MemAccess, MemResult, Memory};

/// A transparent wrapper around memory objects that logs any reads / writes.
///
/// **This should only be used for debugging**!
#[derive(Debug)]
pub struct MemLogger<M: Device>(M);

impl<M: Device> MemLogger<M> {
    pub fn new(memory: M) -> MemLogger<M> {
        MemLogger(memory)
    }
}

macro_rules! impl_memlogger_r {
    ($fn:ident, $ret:ty) => {
        fn $fn(&mut self, offset: u32) -> MemResult<$ret> {
            let val = (self.0).$fn(offset)?;
            info!(
                "[{}] {}",
                Probe::Device {
                    device: self,
                    next: Box::new(self.probe(offset))
                },
                MemAccess::$fn(offset, val)
            );
            Ok(val)
        }
    };
}

macro_rules! impl_memlogger_w {
    ($fn:ident, $val:ty) => {
        fn $fn(&mut self, offset: u32, val: $val) -> MemResult<()> {
            info!(
                "[{}] {}",
                Probe::Device {
                    device: self,
                    next: Box::new(self.probe(offset))
                },
                MemAccess::$fn(offset, val)
            );
            (self.0).$fn(offset, val)?;
            Ok(())
        }
    };
}

impl<M: Device> Device for MemLogger<M> {
    fn kind(&self) -> &'static str {
        self.0.kind()
    }

    fn label(&self) -> Option<&str> {
        self.0.label()
    }

    fn probe(&self, offset: u32) -> Probe<'_> {
        self.0.probe(offset)
    }
}

impl<M: Memory + Device> Memory for MemLogger<M> {
    impl_memlogger_r!(r8, u8);
    impl_memlogger_r!(r16, u16);
    impl_memlogger_r!(r32, u32);
    impl_memlogger_w!(w8, u8);
    impl_memlogger_w!(w16, u16);
    impl_memlogger_w!(w32, u32);
}
