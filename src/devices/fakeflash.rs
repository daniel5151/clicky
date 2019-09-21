use crate::memory::WordAlignedMemory;

/// Just enough Flash ROM for HLE Boot
pub struct FakeFlash {}

impl FakeFlash {
    pub fn new() -> FakeFlash {
        FakeFlash {}
    }
}

impl WordAlignedMemory for FakeFlash {
    fn r32(&mut self, offset: u32) -> u32 {
        assert!(offset < 0x1000_0000);
        match offset {
            // idk what ipodloader/tools.c:get_ipod_rev() is doing lol
            0x2000 => 0xDEAD_BEEF,
            // hardware revision magic number
            // see: https://www.rockbox.org/wiki/IpodHardwareInfo
            0x405c => 0x50000, // iPod 4th Gen
            _ => panic!("accessed unknown Flash address @ offset {:#010x}", offset),
        }
    }
    fn w32(&mut self, offset: u32, _: u32) {
        assert!(offset < 0x1000_0000);
        // noop
    }
}
