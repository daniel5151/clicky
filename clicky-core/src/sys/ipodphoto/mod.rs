use std::io::{Read, Seek};
use std::time::Duration;

use armv4t_emu::{reg, Cpu};
use relativity::Timeout;
use thiserror::Error;

use crate::block::BlockDev;
use crate::devices::Device;
use crate::devices::display::LcdPanel;
use crate::error::*;
use crate::executor::*;
use crate::gui::{IpodBinds, IpodControls, IpodKey, RenderCallback, TakeControls};
use crate::memory::{armv4t_adaptor::MemoryAdapter, Memory};
use crate::signal::{self, gpio, irq};

use crate::sys::{BootKind, System};

use crate::devices::platform::pp::common::*;
use crate::board_mmap;
mod devices {
    pub mod i2c {
        pub use crate::devices::i2c::devices::Pcf5060x;
    }

    pub use crate::devices::{
        display::lr38825::LR38825,
        generic::{ide, AsanRam},
        input::{clickwheel::ClickWheel, Controls},
        platform::pp::*,
    };
}

/// How "--hold-keys" are held down for
const BOOT_HOLD_DURATION: Duration = Duration::from_millis(3000);

/// A IpodPhoto system
#[derive(Debug)]
pub struct IpodPhoto {
    frozen: bool, // set after a fatal error to enable post-mortem debugging

    cpu: Cpu,
    cop: Cpu,
    devices: IpodPhotoBoard,
    controls: Option<IpodControls>,
    /// A second set of keypad signal masters, used to synthesize key presses
    /// independently of whoever took ownership of the system's controls.
    synthetic_controls: devices::Controls<signal::Master>,
    /// Keys to hold down once the system starts executing code.
    boot_hold: Option<Vec<IpodKey>>,

    irq_pending: irq::Pending,
    dma_pending: irq::Pending,
    gpio_changed: gpio::Changed,
    i2c_changed: signal::Trigger,
    reset_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,

    executor: Executor,
}

/// Helper function for calling vectors of EVP
fn vector_via_evp(core: &mut Cpu, devices: &IpodPhotoBoard, vector: u32) {
    if devices.soc.cachecon.local_evt {
        core.reg_set(core.mode(), reg::PC, vector);
    }
}

#[derive(Error, Debug)]
pub enum IpodPhotoBuildError {
    #[error("invalid flash dump: {0}")]
    InvalidDump(&'static str),
    #[error("HLE boot isn't supported on the iPod photo")]
    HleUnsupported,
}

impl IpodPhoto {
    /// Returns a new IpodPhoto instance.
    pub fn new<F>(
        hdd: Box<dyn BlockDev>,
        flash_rom: Option<Box<[u8]>>,
        boot_kind: BootKind<F>,
    ) -> Result<IpodPhoto, IpodPhotoBuildError>
    where
        F: Read + Seek,
    {
        if let BootKind::HLEBoot { .. } = boot_kind {
            return Err(IpodPhotoBuildError::HleUnsupported);
        }

        let executor = Executor::new().expect("failed to create task executor");

        // initialize base system
        let irq_pending = irq::Pending::new();
        let dma_pending = irq::Pending::new();
        let gpio_changed = gpio::Changed::new();
        let i2c_changed = signal::Trigger::new(signal::TriggerKind::Edge);

        // hook-up external controls
        let (mut hold_tx, hold_rx) = gpio::new(gpio_changed.clone(), "Hold");
        let (controls_tx, controls_rx) = devices::Controls::new_tx_rx(i2c_changed.clone());

        let mut sys = IpodPhoto {
            frozen: false,

            cpu: Cpu::new(),
            cop: Cpu::new(),
            devices: IpodPhotoBoard::new(executor.spawner(), irq_pending.clone(), dma_pending.clone()),
            controls: None,
            synthetic_controls: controls_tx.clone(),
            boot_hold: None,

            irq_pending,
            dma_pending,
            gpio_changed: gpio_changed.clone(),
            i2c_changed: i2c_changed.clone(),
            reset_requested: Default::default(),

            executor,
        };

        sys.reset_requested = sys.devices.soc.devcon.reset_requested();

        // connect HDD
        sys.devices
            .soc
            .eidecon
            .as_ide()
            .attach(devices::ide::IdeIdx::IDE0, hdd);

        // Set up flash_rom (if available)
        if let Some(flash_rom) = flash_rom {
            sys.devices
                .flash
                .use_dump(flash_rom)
                .map_err(IpodPhotoBuildError::InvalidDump)?
        }

        {
            let mut gpio_abcd = sys.devices.soc.gpio_abcd.lock().unwrap();
            gpio_abcd.register_in(5, hold_rx.clone());
        }

        {
            let mut clickwheel = devices::ClickWheel::new();
            clickwheel.register_controls(controls_rx, hold_rx);
            sys.devices.soc.opto.attach(Box::new(clickwheel));
        }

        // HACK: Hold is active-low, so set it to high by default
        hold_tx.set_high();

        sys.controls = Some(IpodControls {
            hold: hold_tx,
            controls: controls_tx,
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

        if let Some(keys) = self.boot_hold.take() {
            let mut signals = keys
                .iter()
                .filter_map(|key| crate::gui::key_signal(&self.synthetic_controls, *key))
                .collect::<Vec<_>>();

            self.executor
                .spawner()
                .spawn(async move {
                    for signal in signals.iter_mut() {
                        signal.assert()
                    }

                    Timeout::new(BOOT_HOLD_DURATION).await;

                    for signal in signals.iter_mut() {
                        signal.clear()
                    }
                })
                .expect("failed to spawn boot-hold task");
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
        if self.i2c_changed.check_and_clear() {
            devices.soc.opto.on_change();
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

impl System for IpodPhoto {
    fn model_name(&self) -> &'static str {
        "iPod photo"
    }

    fn screen_size(&self) -> (usize, usize) {
        (220, 176)
    }

    fn set_hold_keys(&mut self, keys: Vec<IpodKey>) {
        if !keys.is_empty() {
            self.boot_hold = Some(keys);
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
        self.devices.soc.clcd.render_callback().expect("no LCD controller attached to the color LCD bridge")
    }
}

impl TakeControls for IpodPhoto {
    fn take_controls(&mut self) -> Option<IpodBinds> {
        Some(self.controls.take()?.into_binds())
    }
}

/// The IpodPhoto board.
///
/// This struct is the "top-level" implementation of the [Memory] trait for the
/// IpodPhoto. It owns the board-level hardware, and layers it over the PP502x SoC
/// that provides everything else.
#[derive(Debug)]
pub struct IpodPhotoBoard {
    pub soc: devices::Pp502x,

    pub sdram: devices::AsanRam,
    pub flash: devices::Flash,
}

impl IpodPhotoBoard {
    fn new(
        task_spawner: Spawner,
        irq_pending: irq::Pending,
        dma_pending: irq::Pending,
    ) -> IpodPhotoBoard {
        use devices::*;

        let mut soc = Pp502x::new(task_spawner, irq_pending, dma_pending);

        soc.i2ccon
            .register_device(0x08, Box::new(i2c::Pcf5060x::new()));

        let lcd_panel = LcdPanel {
            width: 220,
            height: 176,
            reverse_hor: false,
        };
        let mut lcd = LR38825::new();
        lcd.attach(lcd_panel);
        soc.clcd.attach(Box::new(lcd));

        IpodPhotoBoard {
            soc,

            sdram: AsanRam::new(32 * 1024 * 1024, true), // 32 MB
            flash: Flash::new(),
        }
    }
}

board_mmap! {
    IpodPhotoBoard, "IpodPhoto", soc;

    RAM {
        0x1000_0000..=0x11ff_ffff => sdram,
    }

    DEVICES {
        0x0000_0000..=0x000f_ffff => flash,
    }
}
