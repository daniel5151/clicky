use crate::devices::prelude::*;

#[derive(Debug)]
pub struct Firewire {}

impl Firewire {
    pub fn new() -> Firewire {
        Firewire {}
    }
}

impl Device for Firewire {
    fn kind(&self) -> &'static str {
        "Firewire"
    }

    fn probe(&self, _offset: u32) -> Probe {
        Probe::Unmapped
    }
}

impl Memory for Firewire {
    fn r32(&mut self, _offset: u32) -> MemResult<u32> {
        Err(Unimplemented)
    }

    fn w32(&mut self, offset: u32, _val: u32) -> MemResult<()> {
        match offset {
            0x8C => { // Gets sent 0xFFFFFFFF, reverse CFR endianness to BE (3.4.1)?
                Ok(())
            },
            _ => return Err(Unimplemented)
        }
    }
}
