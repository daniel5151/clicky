use std::io::{Read, Seek, SeekFrom};

use armv4t_emu::{reg, Cpu, Mode as ArmMode};

use crate::block::BlockDev;
use crate::devices::{Device, Probe};
use crate::gui::RenderCallback;
use crate::irq;
use crate::memory::{
    armv4t_adaptor::{MemoryAdapter, MemoryAdapterException},
    MemAccess, MemAccessKind, MemException, MemResult, Memory,
};

mod firmware;
mod gdb;
mod hle_bootloader;

mod devices {
    use crate::devices as dev;

    pub use dev::generic;

    pub use dev::generic::asanram::AsanRam;
    pub use dev::generic::stub::Stub;

    pub use dev::cpucon::CpuCon;
    pub use dev::cpuid::{self, CpuId};
    pub use dev::devcon::DevCon;
    pub use dev::eide::EIDECon;
    pub use dev::gpio::{GpioBlock, GpioBlockAtomicMirror};
    pub use dev::hd66753::Hd66753;
    pub use dev::hle_flash::HLEFlash;
    pub use dev::i2c::I2CCon;
    pub use dev::intcon::IntCon;
    pub use dev::memcon::{self, MemCon};
    pub use dev::piezo::Piezo;
    pub use dev::ppcon::PPCon;
    pub use dev::timers::Timers;
}

use crate::devices::util::arcmutex::ArcMutexDevice;

#[derive(Debug)]
pub struct MemExceptionCtx {
    pc: u32,
    access: MemAccess,
    in_device: String,
}

#[derive(Debug)]
pub enum SysError {
    FatalMemException {
        context: MemExceptionCtx,
        reason: MemException,
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
    irq_pending: irq::Pending,
}

impl Ipod4g {
    /// Returns a new PP5020System using High Level Emulation (HLE) of the
    /// bootloader (i.e: without requiring a Flash dump).
    pub fn new_hle(
        mut fw_file: impl Read + Seek,
        hdd: Box<dyn BlockDev>,
    ) -> Result<Ipod4g, Box<dyn std::error::Error>> {
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
        let cop = cpu;

        // create the interrupt bus
        let irq_pending = irq::Pending::new();

        // initialize system devices (in HLE state)
        let mut bus = Ipod4gBus::new_hle(irq_pending.clone());

        // extract image from firmware
        fw_file.seek(SeekFrom::Start(os_image.dev_offset as u64 + 0x200))?;
        let mut os_image_data = vec![0; os_image.len as usize];
        fw_file.read_exact(&mut os_image_data)?;

        bus.sdram.bulk_write(0, &os_image_data);

        // connect HDD
        bus.eidecon
            .as_ide()
            .attach(devices::generic::ide::IdeIdx::IDE0, hdd);

        // inject fake sysinfo into fastram.
        //
        // I threw my copy of the iPod 4g flashROM into Ghidra, and as far as I can
        // tell, the bootloader does indeed set this structure up somewhere in memory.
        // I don't _fully_ understand where ipodloader got this magic pointer address
        // from, because perusing the flashROM disassembly didn't reveal any immediately
        // obvious writes to that address.
        //
        // Anyhoo, I kinda gave up on doing it "correctly," and kinda just futzed around
        // with the addresses until the code managed to progress further. I _hope_ this
        // structure isn't used past the init stage, since I picked the memory location
        // to write it into somewhat arbitrarily, and there's no reason some other code
        // might not come in and trash it...
        //
        // TODO: add some sort of signaling system if the sysinfo struct is overwritten
        use hle_bootloader::sysinfo_t;
        const SYSINFO_PTR: u32 = 0x4001_7f1c;
        // SYSINFO_LOC is pulled out of my ass lol
        const SYSINFO_LOC: u32 = 0x4001_7f00 - std::mem::size_of::<sysinfo_t>() as u32;
        bus.w32(SYSINFO_PTR, SYSINFO_LOC).unwrap(); // pointer to sysinfo
        bus.fastram.bulk_write(
            SYSINFO_LOC - 0x4000_0000,
            // FIXME?: this will break on big-endian systems
            bytemuck::bytes_of(&sysinfo_t {
                IsyS: u32::from_le_bytes(*b"IsyS"),
                len: 0x184,
                boardHwSwInterfaceRev: 0x50000,
                ..Default::default()
            }),
        );

        Ok(Ipod4g {
            hle: true,
            frozen: false,
            cpu,
            cop,
            devices: bus,
            irq_pending,
        })
    }

    fn handle_mem_exception(
        cpu: &Cpu,
        mem: &impl Device,
        exception: MemoryAdapterException,
    ) -> Result<(), SysError> {
        let MemoryAdapterException { access, mem_except } = exception;

        let pc = cpu.reg_get(ArmMode::User, reg::PC);
        let in_mem_space_of = format!("{}", mem.probe(access.offset));

        let ctx = MemExceptionCtx {
            pc,
            access,
            in_device: in_mem_space_of,
        };

        let ctx_str = format!(
            "[pc {:#010x?}][addr {:#010x?}][{}]",
            ctx.pc, access.offset, ctx.in_device
        );

        use MemException::*;
        match mem_except {
            Unimplemented | Unexpected => {
                return Err(SysError::FatalMemException {
                    context: ctx,
                    reason: mem_except,
                })
            }
            StubRead(level, _) => log!(level, "{} stubbed read ({})", ctx_str, access.val),
            StubWrite(level, ()) => log!(level, "{} stubbed write ({})", ctx_str, access.val),
            FatalError(_) => {
                return Err(SysError::FatalMemException {
                    context: ctx,
                    reason: mem_except,
                })
            }

            Misaligned => {
                // FIXME: Misaligned access (i.e: Data Abort) should be a CPU exception.
                return Err(SysError::FatalMemException {
                    context: ctx,
                    reason: mem_except,
                });
            }
            InvalidAccess => match access.kind {
                MemAccessKind::Read => error!("{} read from write-only register", ctx_str),
                MemAccessKind::Write => error!("{} write to read-only register", ctx_str),
            },
            MmuViolation => {
                return Err(SysError::FatalMemException {
                    context: ctx,
                    reason: mem_except,
                })
            }
            ContractViolation {
                msg,
                severity,
                stub_val,
            } => {
                // TODO: use config option to decide if Error-level ContractViolation should
                // terminate execution
                if severity == log::Level::Error {
                    return Err(SysError::FatalMemException {
                        context: ctx,
                        reason: ContractViolation {
                            msg,
                            severity,
                            stub_val,
                        },
                    });
                } else {
                    log!(severity, "{} {}", ctx_str, msg)
                }
            }
        }

        Ok(())
    }

    fn check_device_interrupts(&mut self, _blocking: BlockMode) {
        // use armv4t_emu::Exception;

        // TODO
        if self.irq_pending.has_pending() {
            panic!("IRQ handling isn't implemented yet!");
        }
    }

    /// Run the system for a single CPU instruction, returning `true` if the
    /// system is still running, or `false` upon exiting to the bootloader.
    pub fn step(
        &mut self,
        _log_memory_access: impl FnMut(MemAccess),
        _halt_block_mode: BlockMode,
    ) -> Result<bool, SysError> {
        if self.frozen {
            return Ok(true);
        }

        let run_cpu = self.devices.cpucon.is_cpu_running();
        let run_cop = self.devices.cpucon.is_cop_running();

        // XXX: armv4t_emu doesn't currently expose any way to differentiate between
        // instruction-fetch reads, and regular reads. Therefore, it's impossible to
        // enforce MMU "execute" protection bits...

        if run_cpu {
            self.devices.cpuid.set_cpuid(devices::cpuid::CpuIdKind::Cpu);
            let mut mem = MemoryAdapter::new(&mut self.devices);
            self.cpu.step(&mut mem);
            if let Some(e) = mem.exception.take() {
                Ipod4g::handle_mem_exception(&self.cpu, &self.devices, e)?;
            }
        }

        if run_cop {
            self.devices.cpuid.set_cpuid(devices::cpuid::CpuIdKind::Cop);
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
    pub fn run(&mut self) -> Result<(), SysError> {
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

    /// Return the system's RenderCallback method.
    pub fn render_callback(&self) -> RenderCallback {
        self.devices.hd66753.render_callback()
    }
}

/// The main Ipod4g memory bus.
///
/// This struct is the "top-level" implementation of the [Memory] trait for the
/// Ipod4g, and maps the entire 32 bit address space to the Ipod4g's various
/// devices.
#[derive(Debug)]
pub struct Ipod4gBus {
    pub sdram: devices::AsanRam,
    pub fastram: devices::AsanRam,
    pub cpuid: devices::CpuId,
    pub flash: devices::HLEFlash,
    pub cpucon: devices::CpuCon,
    pub hd66753: devices::Hd66753,
    pub timers: devices::Timers,
    pub gpio_abcd: ArcMutexDevice<devices::GpioBlock>,
    pub gpio_efgh: ArcMutexDevice<devices::GpioBlock>,
    pub gpio_ijkl: ArcMutexDevice<devices::GpioBlock>,
    pub gpio_mirror_abcd: devices::GpioBlockAtomicMirror,
    pub gpio_mirror_efgh: devices::GpioBlockAtomicMirror,
    pub gpio_mirror_ijkl: devices::GpioBlockAtomicMirror,
    pub i2c: devices::I2CCon,
    pub ppcon: devices::PPCon,
    pub devcon: devices::DevCon,
    pub intcon: devices::IntCon,
    pub eidecon: devices::EIDECon,
    pub memcon: devices::MemCon,
    pub piezo: devices::Piezo,

    pub mystery_irq_con: devices::Stub,
    pub mystery_lcd_con: devices::Stub,
}

impl Ipod4gBus {
    #[allow(clippy::redundant_clone)] // Makes the code cleaner in this case
    fn new_hle(irq_pending: irq::Pending) -> Ipod4gBus {
        let gpio_abcd = ArcMutexDevice::new(GpioBlock::new(["A", "B", "C", "D"]));
        let gpio_efgh = ArcMutexDevice::new(GpioBlock::new(["E", "F", "G", "H"]));
        let gpio_ijkl = ArcMutexDevice::new(GpioBlock::new(["I", "J", "K", "L"]));

        let gpio_mirror_abcd = gpio_abcd.clone();
        let gpio_mirror_efgh = gpio_efgh.clone();
        let gpio_mirror_ijkl = gpio_ijkl.clone();

        let (ide_irq_tx, ide_irq_rx) = irq::new(irq_pending, "IDE");

        let mut intcon = IntCon::new_hle();
        intcon.register(23, ide_irq_rx);

        use devices::*;
        Ipod4gBus {
            sdram: AsanRam::new(32 * 1024 * 1024), // 32 MB
            fastram: AsanRam::new(96 * 1024),      // 96 KB
            cpuid: CpuId::new(),
            flash: HLEFlash::new_hle(),
            cpucon: CpuCon::new_hle(),
            hd66753: Hd66753::new_hle(),
            timers: Timers::new_hle(),
            gpio_abcd,
            gpio_efgh,
            gpio_ijkl,
            gpio_mirror_abcd: GpioBlockAtomicMirror::new(gpio_mirror_abcd),
            gpio_mirror_efgh: GpioBlockAtomicMirror::new(gpio_mirror_efgh),
            gpio_mirror_ijkl: GpioBlockAtomicMirror::new(gpio_mirror_ijkl),
            i2c: I2CCon::new_hle(),
            ppcon: PPCon::new_hle(),
            devcon: DevCon::new_hle(),
            intcon,
            eidecon: EIDECon::new_hle(ide_irq_tx),
            memcon: MemCon::new_hle(),
            piezo: Piezo::new(),

            mystery_irq_con: Stub::new("Mystery IRQ Con?"),
            mystery_lcd_con: Stub::new("Mystery LCD Con?"),
        }
    }
}

macro_rules! mmap {
    ($($start:literal ..= $end:literal => $device:ident,)*) => {
        macro_rules! impl_mem_r {
            ($fn:ident, $ret:ty) => {
                fn $fn(&mut self, addr: u32) -> MemResult<$ret> {
                    let (addr, prot) = self.memcon.virt_to_phys(addr);
                    if !prot.r {
                        return Err(MemException::MmuViolation)
                    }

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
                    let (addr, prot) = self.memcon.virt_to_phys(addr);
                    if !prot.w {
                        return Err(MemException::MmuViolation)
                    }

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

            fn probe(&self, addr: u32) -> Probe {
                let (addr, _) = self.memcon.virt_to_phys(addr);
                match addr {
                    $($start..=$end => {
                        Probe::from_device(&self.$device, addr - $start)
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
    0x1000_0000..=0x11ff_ffff => sdram,
    0x4000_0000..=0x4001_7fff => fastram,
    0x6000_0000..=0x6000_0fff => cpuid,
    0x6000_1010..=0x6000_1fff => mystery_irq_con,
    0x6000_4000..=0x6000_41ff => intcon,
    0x6000_5000..=0x6000_5fff => timers,
    0x6000_6000..=0x6000_6fff => devcon,
    0x6000_7000..=0x6000_7fff => cpucon,
    0x6000_d000..=0x6000_d07f => gpio_abcd,
    0x6000_d080..=0x6000_d0ff => gpio_efgh,
    0x6000_d100..=0x6000_d17f => gpio_ijkl,
    0x6000_d800..=0x6000_d87f => gpio_mirror_abcd,
    0x6000_d880..=0x6000_d8ff => gpio_mirror_efgh,
    0x6000_d900..=0x6000_d97f => gpio_mirror_ijkl,
    0x7000_0000..=0x7000_1fff => ppcon,
    0x7000_3000..=0x7000_3fff => hd66753,
    0x7000_a000..=0x7000_a003 => piezo,
    0x7000_a010..=0x7000_a014 => mystery_lcd_con,
    0x7000_c000..=0x7000_cfff => i2c,
    0xc300_0000..=0xc300_0fff => eidecon,
    0xf000_0000..=0xf000_ffff => memcon,
}
