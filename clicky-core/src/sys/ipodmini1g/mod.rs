use std::io::{Read, Seek};

use armv4t_emu::{reg, Cpu};
use thiserror::Error;

use crate::block::BlockDev;
use crate::devices::display::LcdPanel;
use crate::devices::Device;
use crate::error::*;
use crate::executor::*;
use crate::gui::{IpodBinds, IpodKey, RenderCallback, TakeControls};
use crate::memory::{armv4t_adaptor::MemoryAdapter, Memory};
use crate::signal::{gpio, irq};

mod controls;

use crate::sys::{BootKind, System};

use controls::IpodMini1gControls;

use crate::board_mmap;
use crate::devices::platform::pp::common::*;
mod devices {
    pub mod i2c {
        pub use crate::devices::i2c::devices::Pcf5060x;
    }

    pub use crate::devices::{
        display::hd66753::Hd66753,
        generic::{ide, AsanRam},
        platform::pp::*,
    };
}

#[derive(Debug)]
pub struct IpodMini1g {
    frozen: bool, // set after a fatal error to enable post-mortem debugging

    cpu: Cpu,
    cop: Cpu,
    devices: IpodMini1gBoard,
    controls: Option<IpodMini1gControls>,

    irq_pending: irq::Pending,
    dma_pending: irq::Pending,
    gpio_changed: gpio::Changed,
    reset_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,

    executor: Executor,
}

fn vector_via_evp(core: &mut Cpu, devices: &IpodMini1gBoard, vector: u32) {
    if devices.soc.cachecon.local_evt {
        core.reg_set(core.mode(), reg::PC, vector);
    }
}

#[derive(Error, Debug)]
pub enum IpodMini1gBuildError {
    #[error("invalid flash dump: {0}")]
    InvalidDump(&'static str),
    #[error("HLE boot isn't supported on the iPod mini 1g")]
    HleUnsupported,
}

impl IpodMini1g {
    pub fn new<F>(
        hdd: Box<dyn BlockDev>,
        flash_rom: Option<Box<[u8]>>,
        boot_kind: BootKind<F>,
    ) -> Result<IpodMini1g, IpodMini1gBuildError>
    where
        F: Read + Seek,
    {
        if let BootKind::HLEBoot { .. } = boot_kind {
            return Err(IpodMini1gBuildError::HleUnsupported);
        }

        let executor = Executor::new().expect("failed to create task executor");

        // initialize base system
        let irq_pending = irq::Pending::new();
        let dma_pending = irq::Pending::new();
        let gpio_changed = gpio::Changed::new();

        // hook-up external controls
        let (mut hold_tx, hold_rx) = gpio::new(gpio_changed.clone(), "Hold");
        let (mut action_tx, action_rx) = gpio::new(gpio_changed.clone(), "KeyAction");
        let (mut up_tx, up_rx) = gpio::new(gpio_changed.clone(), "KeyUp");
        let (mut down_tx, down_rx) = gpio::new(gpio_changed.clone(), "KeyDown");
        let (mut right_tx, right_rx) = gpio::new(gpio_changed.clone(), "KeyRight");
        let (mut left_tx, left_rx) = gpio::new(gpio_changed.clone(), "KeyLeft");

        let mut sys = IpodMini1g {
            frozen: false,

            cpu: Cpu::new(),
            cop: Cpu::new(),
            devices: IpodMini1gBoard::new(
                executor.spawner(),
                irq_pending.clone(),
                dma_pending.clone(),
            ),
            controls: None,

            irq_pending,
            dma_pending,
            gpio_changed,
            reset_requested: Default::default(),

            executor,
        };

        sys.reset_requested = sys.devices.soc.devcon.reset_requested();

        sys.devices
            .soc
            .eidecon
            .as_ide()
            .attach(devices::ide::IdeIdx::IDE0, hdd);

        if let Some(flash_rom) = flash_rom {
            sys.devices
                .flash
                .use_dump(flash_rom)
                .map_err(IpodMini1gBuildError::InvalidDump)?
        }

        {
            // GPIO A0..=A5
            let mut gpio_abcd = sys.devices.soc.gpio_abcd.lock().unwrap();
            gpio_abcd
                .register_in(0, action_rx)
                .register_in(1, up_rx)
                .register_in(2, down_rx)
                .register_in(3, right_rx)
                .register_in(4, left_rx)
                .register_in(5, hold_rx);
        }

        // Hold and the buttons are all active-low, so set them to high by default
        hold_tx.set_high();
        action_tx.set_high();
        up_tx.set_high();
        down_tx.set_high();
        right_tx.set_high();
        left_tx.set_high();

        sys.controls = Some(IpodMini1gControls {
            hold: hold_tx,
            action: action_tx,
            up: up_tx,
            down: down_tx,
            left: left_tx,
            right: right_tx,
        });

        Ok(sys)
    }

    fn warm_reset(&mut self) {
        self.devices.soc.warm_reset();

        self.cpu = Cpu::new();
        self.cop = Cpu::new();
    }

    /// Run the system for a single CPU instruction, returning `true` if the
    /// system is still running, or `false` upon reaching some sort of "graceful
    /// exit" condition (e.g: power-off).
    fn step(&mut self) -> FatalMemResult<bool> {
        if self.frozen {
            return Ok(true);
        }

        if self
            .reset_requested
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            info!("system reset requested");
            self.warm_reset();
            return Ok(true);
        }

        // TODO: if neither CPU is running, efficiently block until the next IRQ

        let devices = &mut self.devices;
        for (cpu, cpuid) in [(&mut self.cpu, CpuId::Cpu), (&mut self.cop, CpuId::Cop)].iter_mut() {
            if !devices.soc.cpucon.is_cpu_running(*cpuid) {
                continue;
            }

            // XXX: armv4t_emu doesn't currently expose any way to differentiate between
            // instruction-fetch reads, and regular reads. Therefore, it's impossible to
            // enforce MMU "execute" protection bits...

            // FIXME: this approach is kinda gross. Maybe add a some "ctx" to `Memory`?
            devices.soc.cpuid.set_cpuid(*cpuid);
            devices.soc.memcon.set_cpuid(*cpuid);
            devices.soc.mailbox.set_cpuid(*cpuid);

            let mut mem = MemoryAdapter::new(devices);
            cpu.step(&mut mem);
            if let Some((access, e)) = mem.exception.take() {
                e.resolve(
                    "MMIO",
                    MemExceptionCtx {
                        pc: cpu.reg_get(cpu.mode(), reg::PC),
                        access,
                        in_device: format!("{}, {}", cpuid, devices.probe(access.offset)),
                    },
                )?;
            }
        }

        // TODO: don't run this on every cycle?
        self.executor.run_until_stalled();

        // XXX: this is terrible. truly god awful. it _really_ needs to be rewritten,
        // reorganized, and moved somewhere more appropriate.
        if self.dma_pending.check() {
            self.dma_pending.clear();
            if devices.soc.dmacon0.do_ide_dma() {
                let (kind, addr) = match (devices.soc.eidecon).do_dma() {
                    Ok(tup) => tup,
                    Err(_) => panic!("asd"),
                };

                use crate::memory::MemAccessKind;
                match kind {
                    MemAccessKind::Read => {
                        let val = (devices.soc.eidecon.as_ide())
                            .read16(devices::ide::IdeReg::Data)
                            .unwrap();
                        devices.w16(addr, val).unwrap();
                    }
                    MemAccessKind::Write => {
                        let val = devices.r16(addr).unwrap();
                        (devices.soc.eidecon.as_ide())
                            .write16(devices::ide::IdeReg::Data, val)
                            .unwrap();
                    }
                    MemAccessKind::Execute => {
                        panic!("Unsupported execute DMA");
                    }
                }
            }
        }

        // TODO?: explore adding callbacks to the signaling system
        if self.gpio_changed.check_and_clear() {
            devices.soc.gpio_abcd.lock().unwrap().update();
            devices.soc.gpio_efgh.lock().unwrap().update();
            devices.soc.gpio_ijkl.lock().unwrap().update();
        }

        if self.irq_pending.check() {
            use armv4t_emu::Exception;

            let (cpu_status, cop_status) = devices.soc.intcon.interrupt_status();

            for (core, cpuid, status) in [
                (&mut self.cpu, CpuId::Cpu, cpu_status),
                (&mut self.cop, CpuId::Cop, cop_status),
            ]
            .iter_mut()
            {
                if status.irq {
                    devices.soc.cpucon.wake_on_interrupt(*cpuid);
                    let taken = core.irq_enable();
                    core.exception(Exception::Interrupt);
                    if taken {
                        vector_via_evp(core, devices, devices.soc.evp.normal_irq_vec());
                    }

                    if core.irq_enable() {
                        self.irq_pending.clear();
                    }
                }
                if status.fiq {
                    devices.soc.cpucon.wake_on_interrupt(*cpuid);
                    let taken = core.fiq_enable();
                    core.exception(Exception::FastInterrupt);
                    if taken {
                        vector_via_evp(core, devices, devices.soc.evp.high_priority_irq_vec());
                    }

                    if core.fiq_enable() {
                        self.irq_pending.clear();
                    }
                }
            }
        }

        Ok(true)
    }
}

impl System for IpodMini1g {
    fn model_name(&self) -> &'static str {
        "iPod mini 1g"
    }

    fn screen_size(&self) -> (usize, usize) {
        (138, 110)
    }

    fn set_hold_keys(&mut self, keys: Vec<IpodKey>) {
        if !keys.is_empty() {
            // TODO: hold keys down at boot
            warn!("holding keys at boot isn't supported on the iPod mini 1g yet");
        }
    }

    fn run(&mut self) -> FatalMemResult<()> {
        while self.step()? {}
        Ok(())
    }

    fn run_cycles(&mut self, cycles: usize) -> FatalMemResult<()> {
        for _ in 0..cycles {
            self.step()?;
        }
        Ok(())
    }

    fn freeze(&mut self) {
        self.frozen = true;
    }

    fn render_callback(&self) -> RenderCallback {
        self.devices
            .soc
            .mlcd
            .render_callback()
            .expect("no LCD controller attached to the mono LCD bridge")
    }
}

impl TakeControls for IpodMini1g {
    fn take_controls(&mut self) -> Option<IpodBinds> {
        Some(self.controls.take()?.into_binds())
    }
}

#[derive(Debug)]
pub struct IpodMini1gBoard {
    pub soc: devices::Pp502x,

    pub sdram: devices::AsanRam,
    pub flash: devices::Flash,
}

impl IpodMini1gBoard {
    fn new(
        task_spawner: Spawner,
        irq_pending: irq::Pending,
        dma_pending: irq::Pending,
    ) -> IpodMini1gBoard {
        use devices::*;

        let mut soc = Pp502x::new(task_spawner, irq_pending, dma_pending);

        soc.i2ccon
            .register_device(0x08, Box::new(i2c::Pcf5060x::new()));

        let lcd_panel = LcdPanel {
            width: 138,
            height: 110,
            reverse_hor: true,
        };
        soc.mlcd.attach(Box::new(Hd66753::new(lcd_panel)));

        IpodMini1gBoard {
            soc,

            sdram: AsanRam::new(32 * 1024 * 1024, true), // 32 MB
            flash: Flash::new(),
        }
    }
}

board_mmap! {
    IpodMini1gBoard, "IpodMini1g", soc;

    RAM {
        0x1000_0000..=0x11ff_ffff => sdram,
    }

    DEVICES {
        0x0000_0000..=0x000f_ffff => flash,
    }
}
