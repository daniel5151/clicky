use arm7tdmi_rs::Memory;

/// A trait for implementing memory devices that enfore word-aligned access
pub trait WordAlignedMemory {
    /// Read a 32-bit value from the device's base address + `offset`
    fn r32(&mut self, offset: u32) -> u32;
    /// Read a 32-bit `val` to the device's base address + `offset`
    fn w32(&mut self, offset: u32, val: u32);
}

/// A wrapper around [`WordAlignedMemory`] objects that derives
/// [`arm7tdmi::Memory`]. When a misaligned access happens, a `access_violation`
/// flag is set.
pub struct WordAligned<T: WordAlignedMemory> {
    access_violation: bool,
    inner: T,
}

use std::ops::{Deref, DerefMut};

impl<T: WordAlignedMemory> Deref for WordAligned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: WordAlignedMemory> DerefMut for WordAligned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: WordAlignedMemory> WordAligned<T> {
    pub fn new(inner: T) -> WordAligned<T> {
        WordAligned {
            access_violation: false,
            inner,
        }
    }

    /// Checks for an access violation, clearing the boolean once read
    pub fn check_access_violation(&mut self) -> bool {
        let ret = self.access_violation;
        self.access_violation = false;
        ret
    }
}

impl<T> Memory for WordAligned<T>
where
    T: WordAlignedMemory,
{
    /// Read a 8-bit value from `addr`
    fn r8(&mut self, addr: u32) -> u8 {
        if addr & 0x3 != 0 {
            self.access_violation = true;
            0x00 // return dummy value
        } else {
            self.inner.r32(addr) as u8
        }
    }
    /// Read a 16-bit value from `addr`
    fn r16(&mut self, addr: u32) -> u16 {
        if addr & 0x3 != 0 {
            self.access_violation = true;
            0x00 // return dummy value
        } else {
            self.inner.r32(addr) as u16
        }
    }
    /// Read a 32-bit value from `addr`
    fn r32(&mut self, addr: u32) -> u32 {
        self.inner.r32(addr)
    }

    /// Write a 8-bit `val` to `addr`
    fn w8(&mut self, addr: u32, val: u8) {
        if addr & 0x3 != 0 {
            self.access_violation = true;
        } else {
            self.inner.w32(addr, val as u32)
        }
    }
    /// Write a 16-bit `val` to `addr`
    fn w16(&mut self, addr: u32, val: u16) {
        if addr & 0x3 != 0 {
            self.access_violation = true;
        } else {
            self.inner.w32(addr, val as u32)
        }
    }
    /// Write a 32-bit `val` to `addr`
    fn w32(&mut self, addr: u32, val: u32) {
        self.inner.w32(addr, val)
    }
}
