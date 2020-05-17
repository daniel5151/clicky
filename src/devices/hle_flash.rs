use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// A stand-in for the flash ROM present on actual iPods. Is mostly empty, aside
/// from a few memory locations which serve to identify the iPod model.
#[derive(Debug)]
pub struct HLEFlash {}

impl HLEFlash {
    pub fn new_hle() -> HLEFlash {
        HLEFlash {}
    }
}

impl Device for HLEFlash {
    fn kind(&self) -> &'static str {
        "Flash Rom (HLE)"
    }

    fn probe(&self, offset: u32) -> Probe<'_> {
        let reg = match offset {
            0x2000 => "b\"gfCS\" on some iPod revisions",
            0x2084 => "hw revision magic number",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for HLEFlash {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            // idk what ipodloader/tools.c:get_ipod_rev() is doing lol
            0x2000 => Ok(u32::from_le_bytes(*b"gfCS")),
            // hardware revision magic number
            // see: https://www.rockbox.org/wiki/IpodHardwareInfo
            0x2084 => Ok(0x0005_0014), // iPod 4th Gen
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, _val: u32) -> MemResult<()> {
        match offset {
            _ => Err(ContractViolation {
                msg: "tried to write to Flash Rom".to_string(),
                severity: log::Level::Warn,
                stub_val: None,
            }),
        }
    }
}
