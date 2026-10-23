//! Platform support for the PortalPlayer 50xx line of SoCs.

mod mmap;

mod ahb_dma;
mod apb_dma;
mod cachecon;
mod cfg_timer;
mod clcd;
mod cpucon;
mod cpuid;
mod devcon;
mod eide;
mod evp;
mod firewire;
mod flash;
mod gpio;
mod i2c;
mod i2s;
mod intcon;
mod mailbox;
mod memcon;
mod mlcd;
mod opto;
mod ppcon;
mod pwm;
mod rtc;
mod serial;
mod soc;
mod usb;
mod usec_timer;

pub use ahb_dma::*;
pub use apb_dma::*;
pub use cachecon::*;
pub use cfg_timer::*;
pub use clcd::*;
pub use cpucon::*;
pub use cpuid::*;
pub use devcon::*;
pub use eide::*;
pub use evp::*;
pub use firewire::*;
pub use flash::*;
pub use gpio::*;
pub use i2c::*;
pub use i2s::*;
pub use intcon::*;
pub use mailbox::*;
pub use memcon::*;
pub use mlcd::*;
pub use opto::*;
pub use ppcon::*;
pub use pwm::*;
pub use rtc::*;
pub use serial::*;
pub use soc::*;
pub use usb::*;
pub use usec_timer::*;

pub mod common {
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub enum CpuId {
        Cpu,
        Cop,
    }

    impl std::fmt::Display for CpuId {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                CpuId::Cpu => write!(f, "CPU"),
                CpuId::Cop => write!(f, "COP"),
            }
        }
    }
}
