use crate::memory::{MemResult, MemResultExt, Memory};

/// Just enough Flash ROM for HLE Boot
pub struct FakeFlash {}

impl FakeFlash {
    pub fn new() -> FakeFlash {
        FakeFlash {}
    }
}

impl Memory for FakeFlash {
    fn label(&self) -> String {
        "FakeFlash".to_string()
    }

    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        assert!(offset < 0x0010_0000);
        match offset {
            // idk what ipodloader/tools.c:get_ipod_rev() is doing lol
            0x2000 => Ok(0xDEAD_BEEF),
            // hardware revision magic number
            // see: https://www.rockbox.org/wiki/IpodHardwareInfo
            0x405c => Ok(0x50000), // iPod 4th Gen
            _ => crate::unimplemented_offset!(),
        }
        .map_memerr_ctx(offset, self.label())
    }
    fn w32(&mut self, offset: u32, _: u32) -> MemResult<()> {
        assert!(offset < 0x0010_0000);
        // noop
        Ok(())
    }
}
