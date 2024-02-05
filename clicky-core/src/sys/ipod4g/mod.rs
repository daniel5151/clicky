use std::io::{Read, Seek};

use armv4t_emu::{reg, Cpu};
use thiserror::Error;

use crate::block::BlockDev;
use crate::devices::{Device, Probe};
use crate::error::*;
use crate::executor::*;
use crate::gui::RenderCallback;
use crate::memory::{armv4t_adaptor::MemoryAdapter, MemAccess, Memory};
use crate::signal::{self, gpio, irq};

mod controls;
mod gdb;
mod hle_bootloader;

pub use controls::{Ipod4gBinds, Ipod4gKey};
pub use gdb::Ipod4gGdb;

use hle_bootloader::run_hle_bootloader;

use crate::devices::platform::pp::common::*;
use crate::devices::util::{ArcMutexDevice, MemSniffer};
mod devices {
    pub mod i2c {
        pub use crate::devices::i2c::devices::Pcf5060x;
    }

    pub use crate::devices::{
        display::hd66753::Hd66753,
        generic::{ide, AsanRam, Stub},
        platform::pp::*,
    };
}

enum BlockMode {
    Blocking,
    NonBlocking,
}

pub enum BootKind<F: Read + Seek> {
    ColdBoot,
    HLEBoot { fw_file: F },
}

#[derive(Debug)]
struct Ipod4gControls {
    hold: gpio::Sender,
    controls: devices::Controls<signal::Master>,
}

/// A Ipod4g system
#[derive(Debug)]
pub struct Ipod4g {
    frozen: bool,         // set after a fatal error to enable post-mortem debugging
    skip_irq_check: bool, // set by the GDB stub when single-stepping though code

    cpu: Cpu,
    cop: Cpu,
    devices: Ipod4gBus,
    controls: Option<Ipod4gControls>,

    irq_pending: irq::Pending,
    dma_pending: irq::Pending,
    gpio_changed: gpio::Changed,
    i2c_changed: signal::Trigger,

    executor: Executor,
}

#[derive(Error, Debug)]
pub enum Ipod4gBuildError {
    #[error("invalid flash dump: {0}")]
    InvalidDump(&'static str),
    #[error("HLE bootloader failed! {0}")]
    HleBootloader(#[from] hle_bootloader::HleBootloaderError),
}

impl Ipod4g {
    /// Returns a new Ipod4g instance.
    pub fn new<F>(
        hdd: Box<dyn BlockDev>,
        flash_rom: Option<Box<[u8]>>,
        boot_kind: BootKind<F>,
    ) -> Result<Ipod4g, Ipod4gBuildError>
    where
        F: Read + Seek,
    {
        let executor = Executor::new().expect("failed to create task executor");

        // initialize base system
        let irq_pending = irq::Pending::new();
        let dma_pending = irq::Pending::new();
        let gpio_changed = gpio::Changed::new();
        let i2c_changed = signal::Trigger::new(signal::TriggerKind::Edge);

        let mut sys = Ipod4g {
            frozen: false,
            skip_irq_check: false,

            cpu: Cpu::new(),
            cop: Cpu::new(),
            devices: Ipod4gBus::new(executor.spawner(), irq_pending.clone(), dma_pending.clone()),
            controls: None,

            irq_pending,
            dma_pending,
            gpio_changed: gpio_changed.clone(),
            i2c_changed: i2c_changed.clone(),

            executor,
        };

        // connect HDD
        sys.devices
            .eidecon
            .as_ide()
            .attach(devices::ide::IdeIdx::IDE0, hdd);

        // Set up flash_rom (if available)
        if let Some(flash_rom) = flash_rom {
            sys.devices
                .flash
                .use_dump(flash_rom)
                .map_err(Ipod4gBuildError::InvalidDump)?
        }

        // hook-up external controls
        let (mut hold_tx, hold_rx) = gpio::new(gpio_changed, "Hold");
        let (controls_tx, controls_rx) = devices::Controls::new_tx_rx(i2c_changed);

        {
            let mut gpio_abcd = sys.devices.gpio_abcd.lock().unwrap();
            gpio_abcd.register_in(5, hold_rx.clone());
        }

        {
            sys.devices.opto.register_controls(controls_rx, hold_rx)
        }

        // HACK: Hold is active-low, so set it to high by default
        hold_tx.set_high();

        sys.controls = Some(Ipod4gControls {
            hold: hold_tx,
            controls: controls_tx,
        });

        // Run the HLE bootloader if an HLE boot was requested
        if let BootKind::HLEBoot { fw_file } = boot_kind {
            run_hle_bootloader(&mut sys, fw_file)?
        }

        Ok(sys)
    }

    /// Run the system for a single CPU instruction, returning `true` if the
    /// system is still running, or `false` upon reaching some sort of "graceful
    /// exit" condition (e.g: power-off).
    fn step(
        &mut self,
        _halt_block_mode: BlockMode,
        mut sniff_memory: (&[u32], impl FnMut(CpuId, MemAccess)),
    ) -> FatalMemResult<bool> {
        if self.frozen {
            return Ok(true);
        }

        // TODO: if neither CPU is running, efficiently block until the next IRQ

        let devices = &mut self.devices;
        for (cpu, cpuid) in [(&mut self.cpu, CpuId::Cpu), (&mut self.cop, CpuId::Cop)].iter_mut() {
            if !devices.cpucon.is_cpu_running(*cpuid) {
                continue;
            }

            // XXX: armv4t_emu doesn't currently expose any way to differentiate between
            // instruction-fetch reads, and regular reads. Therefore, it's impossible to
            // enforce MMU "execute" protection bits...

            // FIXME: this approach is kinda gross. Maybe add a some "ctx" to `Memory`?
            devices.cpuid.set_cpuid(*cpuid);
            devices.memcon.set_cpuid(*cpuid);
            devices.mailbox.set_cpuid(*cpuid);

            let mut sniffer = MemSniffer::new(devices, sniff_memory.0, |access| {
                sniff_memory.1(*cpuid, access)
            });
            let mut mem = MemoryAdapter::new(&mut sniffer);
            cpu.step(&mut mem);
            if let Some((access, e)) = mem.exception.take() {
                e.resolve(
                    "MMIO",
                    MemExceptionCtx {
                        pc: cpu.reg_get(cpu.mode(), reg::PC),
                        access,
                        in_device: format!("{}", devices.probe(access.offset)),
                    },
                )?;
            }
        }

        if self.skip_irq_check {
            return Ok(true);
        }

        // TODO: don't run this on every cycle?
        self.executor.run_until_stalled();

        // XXX: this is terrible. truly god awful. it _really_ needs to be rewritten,
        // reorganized, and moved somewhere more appropriate.
        if self.dma_pending.check() {
            self.dma_pending.clear();
            if devices.dmacon.do_ide_dma() {
                let (kind, addr) = match (devices.eidecon).do_dma() {
                    Ok(tup) => tup,
                    Err(_) => panic!("asd"),
                };

                use crate::memory::MemAccessKind;
                match kind {
                    MemAccessKind::Read => {
                        let val = (devices.eidecon.as_ide())
                            .read16(devices::ide::IdeReg::Data)
                            .unwrap();
                        devices.w16(addr, val).unwrap();
                    }
                    MemAccessKind::Write => {
                        let val = devices.r16(addr).unwrap();
                        (devices.eidecon.as_ide())
                            .write16(devices::ide::IdeReg::Data, val)
                            .unwrap();
                    }
                }
            }
        }

        // TODO?: explore adding callbacks to the signaling system
        if self.gpio_changed.check_and_clear() {
            devices.gpio_abcd.lock().unwrap().update();
            devices.gpio_efgh.lock().unwrap().update();
            devices.gpio_ijkl.lock().unwrap().update();
        }
        if self.i2c_changed.check_and_clear() {
            devices.opto.on_change();
        }

        if self.irq_pending.check() {
            use armv4t_emu::Exception;

            let (cpu_status, cop_status) = devices.intcon.interrupt_status();

            for (core, cpuid, status) in [
                (&mut self.cpu, CpuId::Cpu, cpu_status),
                (&mut self.cop, CpuId::Cop, cop_status),
            ]
            .iter_mut()
            {
                if status.irq {
                    devices.cpucon.wake_on_interrupt(*cpuid);
                    core.exception(Exception::Interrupt);

                    if core.irq_enable() {
                        self.irq_pending.clear();
                    }
                }
                if status.fiq {
                    devices.cpucon.wake_on_interrupt(*cpuid);
                    core.exception(Exception::FastInterrupt);

                    if core.fiq_enable() {
                        self.irq_pending.clear();
                    }
                }
            }
        }

        Ok(true)
    }

    /// Run the system, returning successfully on "graceful exit"
    /// (e.g: power-off).
    pub fn run(&mut self) -> FatalMemResult<()> {
        let dummy_sniff_memory = |_, _| {};
        while self.step(BlockMode::Blocking, (&[], dummy_sniff_memory))? {}
        Ok(())
    }

    /// Run the system, returning successfully on "graceful exit" (e.g:
    /// power-off). This method will return after the specified number of cycles
    /// have been executed.
    pub fn run_cycles(&mut self, cycles: usize) -> FatalMemResult<()> {
        let dummy_sniff_memory = |_, _| {};
        for _ in 0..cycles {
            self.step(BlockMode::Blocking, (&[], dummy_sniff_memory))?;
        }
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
    pub cpuid: devices::CpuIdReg,
    pub flash: devices::Flash,
    pub cpucon: devices::CpuCon,
    pub hd66753: devices::Hd66753,
    pub timer1: devices::CfgTimer,
    pub timer2: devices::CfgTimer,
    pub usec_timer: devices::UsecTimer,
    pub gpio_abcd: ArcMutexDevice<devices::GpioBlock>,
    pub gpio_efgh: ArcMutexDevice<devices::GpioBlock>,
    pub gpio_ijkl: ArcMutexDevice<devices::GpioBlock>,
    pub gpio_mirror_abcd: devices::GpioBlockAtomicMirror,
    pub gpio_mirror_efgh: devices::GpioBlockAtomicMirror,
    pub gpio_mirror_ijkl: devices::GpioBlockAtomicMirror,
    pub i2ccon: devices::I2CCon,
    pub opto: devices::OptoWheel,
    pub ppcon: devices::PPCon,
    pub devcon: devices::DevCon,
    pub intcon: devices::IntCon,
    pub eidecon: devices::EIDECon,
    pub memcon: devices::MemCon,
    pub cachecon: devices::CacheCon,
    pub i2s: devices::I2SCon,
    pub mailbox: devices::Mailbox,
    pub dmacon: devices::DmaCon,
    pub serial0: devices::Serial,
    pub serial1: devices::Serial,
    pub evp: devices::Evp,

    pub mystery_irq_con: devices::Stub,
    pub mystery_lcd_con: devices::Stub,
    pub mystery_flash_stub: devices::Stub,
    pub firewire: devices::Stub,
    pub total_mystery: devices::Stub,
    pub pwmcon: devices::PWMCon,
}

impl Ipod4gBus {
    #[allow(clippy::redundant_clone)] // Makes the code cleaner in this case
    fn new(
        task_spawner: Spawner,
        irq_pending: irq::Pending,
        dma_pending: irq::Pending,
    ) -> Ipod4gBus {
        let (ide_irq_tx, ide_irq_rx) = irq::new(irq_pending.clone(), "IDE");
        let (timer1_irq_tx, timer1_irq_rx) = irq::new(irq_pending.clone(), "Timer1");
        let (timer2_irq_tx, timer2_irq_rx) = irq::new(irq_pending.clone(), "Timer2");
        let (gpio0_irq_tx, gpio0_irq_rx) = irq::new(irq_pending.clone(), "GPIO0");
        let (gpio1_irq_tx, gpio1_irq_rx) = irq::new(irq_pending.clone(), "GPIO1");
        let (gpio2_irq_tx, gpio2_irq_rx) = irq::new(irq_pending.clone(), "GPIO2");
        let (i2c_irq_tx, i2c_irq_rx) = irq::new(irq_pending.clone(), "I2C");

        let (ide_dmarq_tx, ide_dmarq_rx) = irq::new(dma_pending.clone(), "IDE DMA");

        // mailbox is the only core-specific IRQ in the system, which is kinda neat
        let (mbx_cpu_irq_tx, mbx_cpu_irq_rx) = irq::new(irq_pending.clone(), "Mailbox (CPU)");
        let (mbx_cop_irq_tx, mbx_cop_irq_rx) = irq::new(irq_pending.clone(), "Mailbox (COP)");

        let gpio_abcd = ArcMutexDevice::new(GpioBlock::new(gpio0_irq_tx, ["A", "B", "C", "D"]));
        let gpio_efgh = ArcMutexDevice::new(GpioBlock::new(gpio1_irq_tx, ["E", "F", "G", "H"]));
        let gpio_ijkl = ArcMutexDevice::new(GpioBlock::new(gpio2_irq_tx, ["I", "J", "K", "L"]));

        let gpio_mirror_abcd = gpio_abcd.clone();
        let gpio_mirror_efgh = gpio_efgh.clone();
        let gpio_mirror_ijkl = gpio_ijkl.clone();

        let mut intcon = IntCon::new();
        intcon
            .register(0, timer1_irq_rx)
            .register(1, timer2_irq_rx)
            .register_core_specific(4, mbx_cpu_irq_rx, mbx_cop_irq_rx)
            // .register(10, i2s_irq_rx)
            // .register(20, usb_irq_rx)
            .register(23, ide_irq_rx)
            // .register(25, firewire_irq_rx)
            // .register(26, dma_irq_rx)
            .register(32, gpio0_irq_rx)
            .register(33, gpio1_irq_rx)
            .register(34, gpio2_irq_rx)
            // .register(36, ser0_irq_rx)
            // .register(37, ser1_irq_rx)
            .register(40, i2c_irq_rx);

        let dmacon = DmaCon::new(ide_dmarq_rx);

        let mut i2ccon = I2CCon::new(i2c_irq_tx.clone());
        i2ccon.register_device(0x08, Box::new(i2c::Pcf5060x::new()));

        use devices::*;
        Ipod4gBus {
            sdram: AsanRam::new(32 * 1024 * 1024, true), // 32 MB
            fastram: AsanRam::new(96 * 1024, true),      // 96 KB
            cpuid: CpuIdReg::new(),
            flash: Flash::new(),
            cpucon: CpuCon::new(task_spawner.clone()),
            hd66753: Hd66753::new(),
            timer1: CfgTimer::new("1", timer1_irq_tx, task_spawner.clone()),
            timer2: CfgTimer::new("2", timer2_irq_tx, task_spawner),
            usec_timer: UsecTimer::new(),
            gpio_abcd,
            gpio_efgh,
            gpio_ijkl,
            gpio_mirror_abcd: GpioBlockAtomicMirror::new(gpio_mirror_abcd),
            gpio_mirror_efgh: GpioBlockAtomicMirror::new(gpio_mirror_efgh),
            gpio_mirror_ijkl: GpioBlockAtomicMirror::new(gpio_mirror_ijkl),
            i2ccon,
            opto: OptoWheel::new(i2c_irq_tx),
            ppcon: PPCon::new(),
            devcon: DevCon::new(),
            intcon,
            eidecon: EIDECon::new(ide_irq_tx, ide_dmarq_tx),
            memcon: MemCon::new(),
            cachecon: CacheCon::new(),
            i2s: I2SCon::new(),
            mailbox: Mailbox::new(mbx_cpu_irq_tx, mbx_cop_irq_tx),
            dmacon,
            serial0: Serial::new("0"),
            serial1: Serial::new("1"),
            evp: Evp::new(),

            mystery_irq_con: Stub::new("Mystery IRQ Con?"),
            mystery_lcd_con: Stub::new("Mystery LCD Con?"),
            mystery_flash_stub: Stub::new("Mystery FlashROM Con?"),
            firewire: Stub::new("Firewire Con?"),
            total_mystery: Stub::new("<total mystery>"),
            pwmcon: PWMCon::new(),
        }
    }
}

macro_rules! mmap {
    (
        RAM {
            $($start_ram:literal $(..= $end_ram:literal)? => $ram:ident,)*
        }
        DEVICES {
            $($start_dev:literal $(..= $end_dev:literal)? => $dev:ident,)*
        }
    ) => {
        macro_rules! impl_mem_r {
            ($fn:ident, $ret:ty) => {
                fn $fn(&mut self, addr: u32) -> MemResult<$ret> {
                    let mut addr = addr;
                    if (0x00..0x1F).contains(&addr) && self.cachecon.local_evt {
                        addr = addr | 0x6000_f100;
                    }

                    let (mut addr, prot) = self.memcon.virt_to_phys(addr);
                    if !prot.r {
                        return Err(MemException::MmuViolation)
                    }

                    match addr {
                        $($start_ram$(..=$end_ram)? => self.$ram.$fn(addr - $start_ram),)*
                        $($start_dev$(..=$end_dev)? => self.$dev.$fn(addr - $start_dev),)*
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
                        $($start_ram$(..=$end_ram)? => self.$ram.$fn(addr - $start_ram, val),)*
                        $($start_dev$(..=$end_dev)? => self.$dev.$fn(addr - $start_dev, val),)*
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
                    $($start_ram$(..=$end_ram)? => {
                        Probe::from_device(&self.$ram, addr - $start_ram)
                    })*
                    $($start_dev$(..=$end_dev)? => {
                        Probe::from_device(&self.$dev, addr - $start_dev)
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
    RAM {
        0x1000_0000..=0x11ff_ffff => sdram,
        0x4000_0000..=0x4001_7fff => fastram,
    }

    DEVICES {
        0x0000_0000..=0x000f_ffff => flash,
        0x6000_0000..=0x6000_0fff => cpuid,
        0x6000_1000..=0x6000_102f => mailbox,
        0x6000_4000..=0x6000_41ff => intcon,
        0x6000_5000..=0x6000_5007 => timer1,
        0x6000_5008..=0x6000_500f => timer2,
        0x6000_5010..=0x6000_5013 => usec_timer,
        0x6000_6000..=0x6000_6fff => devcon,
        0x6000_7000..=0x6000_7fff => cpucon,
        0x6000_a000..=0x6000_bfff => dmacon,
        0x6000_c000..=0x6000_cfff => cachecon,
        0x6000_d000..=0x6000_d07f => gpio_abcd,
        0x6000_d080..=0x6000_d0ff => gpio_efgh,
        0x6000_d100..=0x6000_d17f => gpio_ijkl,
        0x6000_d800..=0x6000_d87f => gpio_mirror_abcd,
        0x6000_d880..=0x6000_d8ff => gpio_mirror_efgh,
        0x6000_d900..=0x6000_d97f => gpio_mirror_ijkl,

        0x6400_4000..=0x6400_41ff => intcon, // i guess there's a mirror?

        0x7000_0000..=0x7000_1fff => ppcon,
        0x7000_3000..=0x7000_301f => hd66753,
        0x7000_6000..=0x7000_6020 => serial0,
        0x7000_6040..=0x7000_6060 => serial1,
        0x7000_a000..=0x7000_a03f => pwmcon,
        0x7000_c000..=0x7000_c0ff => i2ccon,
        0x7000_c100..=0x7000_c1ff => opto,
        0x7000_2800..=0x7000_28ff => i2s,
        0xc300_0000..=0xc300_0fff => eidecon,
        0xf000_0000..=0xf000_ffff => memcon,

        0x6000_f000..=0x6000_f01f => evp, // Tegra drivers mention 0x6000F1xx but 0x6000F0xx is mentioned in PP5020 RE litterature
        0x6000_f100..=0x6000_f11f => evp, // I assume 0x6000F0xx and 0x6000F1xx are mirrored? Maybe one is used for the main CPU,
                                          // the other is used for COP?

        // all the stubs

        0x6000_1038 => mystery_irq_con,
        0x6000_111c => mystery_irq_con,
        0x6000_1128 => mystery_irq_con,
        0x6000_1138 => mystery_irq_con,
        0x6000_3000..=0x6000_30ff => total_mystery,
        0x6000_9000..=0x6000_90ff => total_mystery,
        // Diagnostics program reads from address, and write back 0x10000000
        0x7000_3800 => total_mystery,
        0xc031_b1d8 => mystery_flash_stub,
        0xc031_b1e8 => mystery_flash_stub,
        // Diagnostics program writes 0xffffffff
        0xc600_008c => firewire,
        0xffff_fe00..=0xffff_ffff => mystery_flash_stub,
    }
}
