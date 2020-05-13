use std::io::{Read, Seek, SeekFrom};

use armv4t_emu::{reg, Cpu, Mode as ArmMode};
use crossbeam_channel as chan;
use log::*;

use crate::devices::{Device, Probe};
use crate::memory::{
    armv4t_adaptor::{MemoryAdapter, MemoryAdapterException},
    MemAccess, MemAccessKind, MemException, MemResult, Memory,
};

mod firmware;
mod gdb;

mod devices {
    pub use crate::devices::asanram::{self, AsanRam};
    pub use crate::devices::hle_flash::{self, HLEFlash};
    pub use crate::devices::syscon::{self, SysCon};
}

use crate::devices::syscon::Interrupt;

#[derive(Debug)]
pub enum FatalError {
    FatalMemException {
        addr: u32,
        in_mem_space_of: String,
        reason: MemException,
    },
    ContractViolation {
        in_mem_space_of: String,
        msg: String,
    },
}

pub enum BlockMode {
    Blocking,
    NonBlocking,
}

/// A Ipod4g system
#[derive(Debug)]
pub struct Ipod4g {
    hle: bool,
    frozen: bool,
    cpu: Cpu,
    cop: Cpu,
    devices: Ipod4gBus,
    interrupt_bus: chan::Receiver<(Interrupt, bool)>,
}

impl Ipod4g {
    /// Returns a new PP5020System using High Level Emulation (HLE) of the
    /// bootloader (i.e: without requiring a Flash dump).
    pub fn new_hle(mut fw_file: impl Read + Seek) -> Result<Ipod4g, Box<dyn std::error::Error>> {
        let fw_info = firmware::FirmwareMeta::parse(&mut fw_file)?;

        println!("Parsed firmware meta: {:#x?}", fw_info);

        let os_image = fw_info
            .images
            .iter()
            .find(|img| img.name == *b"osos")
            .ok_or("could not find OS image")?;

        // fake the bootloader, load directly at the image address
        let mut cpu = Cpu::new();
        cpu.reg_set(
            ArmMode::User,
            reg::PC,
            os_image.addr + os_image.entry_offset,
        );
        cpu.reg_set(ArmMode::User, reg::CPSR, 0xd3); // supervisor mode
        let cop = cpu.clone();

        // create the interrupt bus
        let (interrupt_bus_tx, interrupt_bus_rx) = chan::unbounded();

        // initialize system devices (in HLE state)
        let mut bus = Ipod4gBus::new_hle(interrupt_bus_tx);

        // extract image from firmware
        fw_file.seek(SeekFrom::Start(os_image.dev_offset as u64 + 0x200))?;
        let mut os_image_data = vec![0; os_image.len as usize];
        fw_file.read_exact(&mut os_image_data)?;

        bus.sdram.bulk_write(0, &os_image_data);

        Ok(Ipod4g {
            hle: true,
            frozen: false,
            cpu,
            cop,
            devices: bus,
            interrupt_bus: interrupt_bus_rx,
        })
    }

    fn handle_mem_exception(
        cpu: &Cpu,
        mem: &impl Device,
        exception: MemoryAdapterException,
    ) -> Result<(), FatalError> {
        let MemoryAdapterException {
            addr,
            kind,
            mem_except,
        } = exception;

        let pc = cpu.reg_get(ArmMode::User, reg::PC);
        let in_mem_space_of = format!("{}", mem.probe(addr));

        let ctx = format!(
            "[pc {:#010x?}][addr {:#010x?}][{}]",
            pc, addr, in_mem_space_of
        );

        use MemException::*;
        match mem_except {
            Unimplemented | Unexpected => {
                return Err(FatalError::FatalMemException {
                    addr,
                    in_mem_space_of,
                    reason: mem_except,
                })
            }
            StubRead(_) => warn!("{} stubbed read", ctx),
            StubWrite => warn!("{} stubbed write", ctx),
            Misaligned => {
                // FIXME: Misaligned access (i.e: Data Abort) should be a CPU exception.
                return Err(FatalError::FatalMemException {
                    addr,
                    in_mem_space_of,
                    reason: mem_except,
                });
            }
            InvalidAccess => match kind {
                MemAccessKind::Read => error!("{} read from write-only register", ctx),
                MemAccessKind::Write => error!("{} write to read-only register", ctx),
            },
            ContractViolation { msg, severity, .. } => {
                if severity == log::Level::Error {
                    return Err(FatalError::ContractViolation {
                        in_mem_space_of,
                        msg,
                    });
                } else {
                    log!(severity, "{} {}", ctx, msg)
                }
            }
        }

        Ok(())
    }

    fn check_device_interrupts(&mut self, _blocking: BlockMode) {
        // use armv4t_emu::Exception;
        // TODO
    }

    /// Run the system for a single CPU instruction, returning `true` if the
    /// system is still running, or `false` upon exiting to the bootloader.
    pub fn step(
        &mut self,
        _log_memory_access: impl FnMut(MemAccess),
        _halt_block_mode: BlockMode,
    ) -> Result<bool, FatalError> {
        if self.frozen {
            return Ok(true);
        }

        let run_cpu = self.devices.syscon.is_cpu_running();
        let run_cop = self.devices.syscon.is_cop_running();

        if run_cpu {
            self.devices.syscon.set_cpuid(devices::syscon::CpuId::Cpu);
            let mut mem = MemoryAdapter::new(&mut self.devices);
            self.cpu.step(&mut mem);
            if let Some(e) = mem.exception.take() {
                Ipod4g::handle_mem_exception(&self.cpu, &self.devices, e)?;
            }
        }

        if run_cop {
            self.devices.syscon.set_cpuid(devices::syscon::CpuId::Cop);
            let mut mem = MemoryAdapter::new(&mut self.devices);
            self.cop.step(&mut mem);
            if let Some(e) = mem.exception.take() {
                Ipod4g::handle_mem_exception(&self.cop, &self.devices, e)?;
            }
        }

        self.check_device_interrupts(BlockMode::NonBlocking);

        Ok(true)
    }

    /// Run the system, returning successfully on "graceful exit".
    ///
    /// In HLE mode, a "graceful exit" is when the PC points into the
    /// bootloader's code.
    pub fn run(&mut self) -> Result<(), FatalError> {
        while self.step(|_| (), BlockMode::Blocking)? {}
        Ok(())
    }

    /// Freeze the system such that `step` becomes a noop. Called prior to
    /// spawning a "post-mortem" GDB session.
    ///
    /// WARNING - THERE IS NO WAY TO "THAW" A FROZEN SYSTEM!
    pub fn freeze(&mut self) {
        self.frozen = true;
    }
}

/// The main Ipod4g memory bus.
///
/// This struct is the "top-level" implementation of the [Memory] trait for the
/// Ipod4g, and maps the entire 32 bit address space to the Ipod4g's various
/// devices.
#[derive(Debug)]
pub struct Ipod4gBus {
    pub sdram: devices::AsanRam,   // 32 MB
    pub fastram: devices::AsanRam, // 32 MB
    pub flash: devices::HLEFlash,
    pub syscon: devices::SysCon,
}

impl Ipod4gBus {
    #[allow(clippy::redundant_clone)] // Makes the code cleaner in this case
    fn new_hle(_interrupt_bus: chan::Sender<(Interrupt, bool)>) -> Ipod4gBus {
        use devices::*;
        Ipod4gBus {
            sdram: AsanRam::new(32 * 1024 * 1024), // 32 MB
            fastram: AsanRam::new(96 * 1024),      // 96 KB
            flash: HLEFlash::new_hle(),
            syscon: SysCon::new_hle(),
        }
    }
}

macro_rules! mmap {
    ($($start:literal ..= $end:literal => $device:ident,)*) => {
        macro_rules! impl_mem_r {
            ($fn:ident, $ret:ty) => {
                fn $fn(&mut self, addr: u32) -> MemResult<$ret> {
                    match addr {
                        $($start..=$end => self.$device.$fn(addr - $start),)*
                        _ => Err(MemException::Unexpected),
                    }
                }
            };
        }

        macro_rules! impl_mem_w {
            ($fn:ident, $val:ty) => {
                fn $fn(&mut self, addr: u32, val: $val) -> MemResult<()> {
                    match addr {
                        $($start..=$end => self.$device.$fn(addr - $start, val),)*
                        _ => Err(MemException::Unexpected),
                    }
                }
            };
        }

        impl Device for Ipod4gBus {
            fn kind(&self) -> &'static str {
                "Ipod4g"
            }

            fn probe(&self, offset: u32) -> Probe {
                match offset {
                    $($start..=$end => {
                        Probe::Device {
                            device: &self.$device,
                            next: Box::new(self.$device.probe(offset - $start))
                        }
                    })*
                    _ => Probe::Unmapped,
                }
            }
        }

        impl Memory for Ipod4gBus {
            impl_mem_r!(r8, u8);
            impl_mem_r!(r16, u16);
            impl_mem_r!(r32, u32);
            impl_mem_w!(w8, u8);
            impl_mem_w!(w16, u16);
            impl_mem_w!(w32, u32);
        }
    };
}

mmap! {
    0x0000_0000..=0x000f_ffff => flash,
    // ???
    0x1000_0000..=0x3fff_ffff => sdram,
    0x4000_0000..=0x4001_7fff => fastram,
    // ???
    0x6000_0000..=0x6fff_ffff => syscon,
}
