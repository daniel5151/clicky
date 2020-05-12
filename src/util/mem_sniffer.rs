use crate::devices::{Device, Probe};
use crate::memory::{MemAccess, MemResult, Memory};

/// [MemSniffer] wraps a [Memory] object, forwarding requests to the underlying
/// memory object, while also recording accesses to the provided callback.
#[derive(Debug)]
pub struct MemSniffer<'a, M, F: FnMut(MemAccess)> {
    mem: &'a mut M,
    on_access: F,
}

impl<'a, M: Memory, F: FnMut(MemAccess)> MemSniffer<'a, M, F> {
    pub fn new(mem: &'a mut M, on_access: F) -> MemSniffer<'a, M, F> {
        MemSniffer { mem, on_access }
    }
}

macro_rules! impl_memsniff_r {
    ($fn:ident, $ret:ty) => {
        fn $fn(&mut self, addr: u32) -> MemResult<$ret> {
            let ret = self.mem.$fn(addr)?;
            (self.on_access)(MemAccess::$fn(addr, ret));
            Ok(ret)
        }
    };
}

macro_rules! impl_memsniff_w {
    ($fn:ident, $val:ty) => {
        fn $fn(&mut self, addr: u32, val: $val) -> MemResult<()> {
            self.mem.$fn(addr, val)?;
            (self.on_access)(MemAccess::$fn(addr, val));
            Ok(())
        }
    };
}

impl<'a, M: Device, F: FnMut(MemAccess)> Device for MemSniffer<'a, M, F> {
    fn kind(&self) -> &'static str {
        self.mem.kind()
    }

    fn label(&self) -> Option<&str> {
        self.mem.label()
    }

    fn probe(&self, offset: u32) -> Probe<'_> {
        self.mem.probe(offset)
    }
}

impl<'a, M: Memory, F: FnMut(MemAccess)> Memory for MemSniffer<'a, M, F> {
    impl_memsniff_r!(r8, u8);
    impl_memsniff_r!(r16, u16);
    impl_memsniff_r!(r32, u32);
    impl_memsniff_w!(w8, u8);
    impl_memsniff_w!(w16, u16);
    impl_memsniff_w!(w32, u32);
}
