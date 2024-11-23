use armv4t_emu::Memory as ArmMemory;

use super::*;

/// The CPU's Memory interface expects all memory accesses to succeed (i.e:
/// return _some_ sort of value). As such, there needs to be some sort of shim
/// between the emulator's fallible [Memory] interface and the CPU's infallible
/// [ArmMemory] interface.
///
/// [MemoryAdapter] wraps a [Memory] object, implementing the [ArmMemory]
/// interface such that if an error occurs while accessing memory, the access
/// will still "succeed", while the exception is stashed away until after the
/// CPU cycle is executed. The `take_exception` method is then be used to check
/// if an exception occurred.
pub struct MemoryAdapter<'a, M: Memory> {
    pub mem: &'a mut M,
    pub exception: Option<(MemAccess, MemException)>,
}

impl<'a, M: Memory> MemoryAdapter<'a, M> {
    pub fn new(mem: &'a mut M) -> Self {
        MemoryAdapter {
            mem,
            exception: None,
        }
    }
}

macro_rules! impl_memadapter_r {
    ($fn:ident, $ret:ty) => {
        fn $fn(&mut self, addr: u32) -> $ret {
            use crate::memory::MemAccessKind;
            match self.mem.$fn(addr) {
                Ok(val) => val,
                Err(e) => {
                    let ret = match e {
                        // If it's a stubbed-read, pass through the stubbed value
                        MemException::StubRead(_, v) => v as $ret,
                        MemException::ContractViolation {
                            stub_val: Some(v), ..
                        } => v as $ret,
                        // otherwise, contents of register undefined
                        _ => 0x00,
                    };
                    // stash the exception
                    self.exception = Some((ret.to_memaccess(addr, MemAccessKind::Read), e));
                    ret
                }
            }
        }
    };
}

macro_rules! impl_memadapter_w {
    ($fn:ident, $val:ty) => {
        fn $fn(&mut self, addr: u32, val: $val) {
            use crate::memory::MemAccessKind;
            match self.mem.$fn(addr, val) {
                Ok(()) => {}
                Err(e) => {
                    // stash the exception
                    self.exception = Some((val.to_memaccess(addr, MemAccessKind::Write), e));
                }
            }
        }
    };
}

macro_rules! impl_memadapter_x {
    ($fn:ident, $ret:ty) => {
        fn $fn(&mut self, addr: u32) -> $ret {
            use crate::memory::MemAccessKind;
            match self.mem.$fn(addr) {
                Ok(val) => val,
                Err(e) => {
                    let ret = match e {
                        // If it's a stubbed-read, pass through the stubbed value
                        MemException::StubRead(_, v) => v as $ret,
                        MemException::ContractViolation {
                            stub_val: Some(v), ..
                        } => v as $ret,
                        // otherwise, contents of register undefined
                        _ => 0x00,
                    };
                    // stash the exception
                    self.exception = Some((ret.to_memaccess(addr, MemAccessKind::Execute), e));
                    ret
                }
            }
        }
    };
}

impl<'a, M: Memory> ArmMemory for MemoryAdapter<'a, M> {
    impl_memadapter_r!(r8, u8);
    impl_memadapter_r!(r16, u16);
    impl_memadapter_r!(r32, u32);
    impl_memadapter_w!(w8, u8);
    impl_memadapter_w!(w16, u16);
    impl_memadapter_w!(w32, u32);
    impl_memadapter_x!(x16, u16);
    impl_memadapter_x!(x32, u32);
}
