//! Platform support for the PortalPlayer 50xx line of SoCs.

pub mod cachecon;
pub mod cpucon;
pub mod cpuid;
pub mod devcon;
pub mod eide;
pub mod flash;
pub mod gpio;
pub mod i2c;
pub mod i2s;
pub mod intcon;
pub mod mailbox;
pub mod memcon;
pub mod piezo;
pub mod ppcon;
pub mod timers;

pub use cachecon::*;
pub use cpucon::*;
pub use cpuid::*;
pub use devcon::*;
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
