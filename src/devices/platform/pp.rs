//! Platform support for the PortalPlayer 50xx line of SoCs.

mod cachecon;
mod cpucon;
mod cpuid;
mod devcon;
mod dma;
mod eide;
mod flash;
mod gpio;
mod i2c;
mod i2s;
mod intcon;
mod mailbox;
mod memcon;
mod piezo;
mod ppcon;
mod timers;

pub use cachecon::*;
pub use cpucon::*;
pub use cpuid::*;
pub use devcon::*;
pub use dma::*;
pub use eide::*;
pub use flash::*;
pub use gpio::*;
pub use i2c::*;
pub use i2s::*;
pub use intcon::*;
pub use mailbox::*;
pub use memcon::*;
pub use piezo::*;
pub use ppcon::*;
pub use timers::*;

pub mod common {
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub enum CpuId {
        Cpu,
        Cop,
    }
}