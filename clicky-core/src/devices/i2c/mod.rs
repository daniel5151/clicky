use crate::devices::prelude::*;

pub mod devices;
pub mod prelude;

pub trait I2CDevice: Device {
    fn read(&mut self) -> MemResult<u8>;
    fn write(&mut self, data: u8) -> MemResult<()>;
}

impl Device for Box<dyn I2CDevice> {
    fn kind(&self) -> &'static str {
        (**self).kind()
    }

    fn label(&self) -> Option<&'static str> {
        (**self).label()
    }

    fn probe(&self, offset: u32) -> Probe {
        (**self).probe(offset)
    }
}
