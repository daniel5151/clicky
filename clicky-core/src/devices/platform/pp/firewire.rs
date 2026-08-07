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

    fn w32(&mut self, _offset: u32, _val: u32) -> MemResult<()> {
        Err(Unimplemented)
    }
}
