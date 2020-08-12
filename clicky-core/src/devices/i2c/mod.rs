use crate::devices::prelude::*;

pub mod devices;
pub mod prelude;

/// Common trait implemented by all i2c devices.
///
/// i2c devices implement the standard `Device` trait, albeit with a slightly
/// different `probe` behavior. Instead of using the provided `offset`, they
/// should instead return the name of whatever internal register was previously
/// selected.
pub trait I2CDevice: Device {
    /// Read an 8-bit value from the device.
    fn read(&mut self) -> MemResult<u8>;
    /// Write an 8-bit value from the device.
    fn write(&mut self, data: u8) -> MemResult<()>;
    /// Called at the end of a write sequence.
    fn write_done(&mut self) -> MemResult<()>;
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
