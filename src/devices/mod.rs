use arm7tdmi_rs::Memory;

pub mod cpuid;

/// A trait for implementing memory-mapped devices with word-aligned addressing.
pub trait DeviceTrait {
    /// Read a 32-bit value from the device's base address + `offset`
    fn r32(&mut self, offset: u32) -> u32;
    /// Read a 32-bit `val` to the device's base address + `offset`
    fn w32(&mut self, offset: u32, val: u32);
}

/// Wrapper around types which implement [DeviceTrait] automatically implement
/// [`arm7tdmi::Memory`] that sets a `access_violation` flag when there is a
/// misaligned access.
pub struct Device<T: DeviceTrait> {
    access_violation: bool,
    device: T,
}

impl<D: DeviceTrait> Device<D> {
    pub fn new(device: D) -> Device<D> {
        Device {
            access_violation: false,
            device,
        }
    }

    /// Checks for an access violation, clearing the boolean once read
    pub fn check_access_violation(&mut self) -> bool {
        let ret = self.access_violation;
        self.access_violation = false;
        ret
    }

    pub fn as_mut(&mut self) -> &mut D {
        &mut self.device
    }

    pub fn as_ref(&self) -> &D {
        &self.device
    }
}

impl<D> Memory for Device<D>
where
    D: DeviceTrait,
{
    /// Read a 8-bit value from `addr`
    fn r8(&mut self, addr: u32) -> u8 {
        if addr & 0x3 != 0 {
            self.access_violation = true;
            0x00 // return dummy value
        } else {
            self.device.r32(addr) as u8
        }
    }
    /// Read a 16-bit value from `addr`
    fn r16(&mut self, addr: u32) -> u16 {
        if addr & 0x3 != 0 {
            self.access_violation = true;
            0x00 // return dummy value
        } else {
            self.device.r32(addr) as u16
        }
    }
    /// Read a 32-bit value from `addr`
    fn r32(&mut self, addr: u32) -> u32 {
        self.device.r32(addr)
    }

    /// Write a 8-bit `val` to `addr`
    fn w8(&mut self, addr: u32, val: u8) {
        if addr & 0x3 != 0 {
            self.access_violation = true;
        } else {
            self.device.w32(addr, val as u32)
        }
    }
    /// Write a 16-bit `val` to `addr`
    fn w16(&mut self, addr: u32, val: u16) {
        if addr & 0x3 != 0 {
            self.access_violation = true;
        } else {
            self.device.w32(addr, val as u32)
        }
    }
    /// Write a 32-bit `val` to `addr`
    fn w32(&mut self, addr: u32, val: u32) {
        self.device.w32(addr, val)
    }
}
