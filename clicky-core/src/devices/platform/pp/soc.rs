use crate::devices::prelude::*;

use crate::devices::generic::{AsanRam, Stub};
use crate::devices::util::ArcMutexDevice;

use super::*;

#[derive(Debug)]
pub struct Pp502x {
    /// On-chip SRAM ("IRAM").
    pub fastram: AsanRam,

    pub cpuid: CpuIdReg,
    pub cpucon: CpuCon,
    pub mlcd: MonoLcdBridge,
    pub timer1: CfgTimer,
    pub timer2: CfgTimer,
    pub usec_timer: UsecTimer,
    pub rtc: Rtc,
    pub firewire: Firewire,
    pub usb: Usb,
    pub gpio_abcd: ArcMutexDevice<GpioBlock>,
    pub gpio_efgh: ArcMutexDevice<GpioBlock>,
    pub gpio_ijkl: ArcMutexDevice<GpioBlock>,
    pub gpio_mirror_abcd: GpioBlockAtomicMirror,
    pub gpio_mirror_efgh: GpioBlockAtomicMirror,
    pub gpio_mirror_ijkl: GpioBlockAtomicMirror,
    pub i2ccon: I2CCon,
    pub opto: OptoWheel,
    pub ppcon: PPCon,
    pub devcon: DevCon,
    pub intcon: IntCon,
    pub eidecon: EIDECon,
    pub memcon: MemCon,
    pub cachecon: CacheCon,
    pub i2s: I2SCon,
    pub mailbox: Mailbox,
    pub dmacon0: DmaCon,
    pub dmacon1: DmaCon,
    pub serial0: Serial,
    pub serial1: Serial,
    pub evp: Evp,
    pub pwmcon: PWMCon,

    pub mystery_irq_con: Stub,
    pub mystery_flash_stub: Stub,
    pub total_mystery: Stub,
    pub pp5002_serial_stub: Stub,
}

impl Pp502x {
    pub fn new(
        task_spawner: Spawner,
        irq_pending: irq::Pending,
        dma_pending: irq::Pending,
    ) -> Pp502x {
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

        let dmacon0 = DmaCon::new("0", Some(ide_dmarq_rx));
        // the undocumented second engine -- nothing routes DMA requests to it yet
        let dmacon1 = DmaCon::new("1", None);

        Pp502x {
            fastram: AsanRam::new(96 * 1024, true), // 96 KB
            cpuid: CpuIdReg::new(),
            cpucon: CpuCon::new(task_spawner.clone()),
            mlcd: MonoLcdBridge::new(),
            timer1: CfgTimer::new("1", timer1_irq_tx, task_spawner.clone()),
            timer2: CfgTimer::new("2", timer2_irq_tx, task_spawner),
            usec_timer: UsecTimer::new(),
            rtc: Rtc::new(),
            firewire: Firewire::new(),
            usb: Usb::new(),
            gpio_abcd,
            gpio_efgh,
            gpio_ijkl,
            gpio_mirror_abcd: GpioBlockAtomicMirror::new(gpio_mirror_abcd),
            gpio_mirror_efgh: GpioBlockAtomicMirror::new(gpio_mirror_efgh),
            gpio_mirror_ijkl: GpioBlockAtomicMirror::new(gpio_mirror_ijkl),
            i2ccon: I2CCon::new(i2c_irq_tx.clone()),
            // opto IRQ line is shared with I2C
            opto: OptoWheel::new(i2c_irq_tx),
            ppcon: PPCon::new(),
            devcon: DevCon::new(),
            intcon,
            eidecon: EIDECon::new(ide_irq_tx, ide_dmarq_tx),
            memcon: MemCon::new(),
            cachecon: CacheCon::new(),
            i2s: I2SCon::new(),
            mailbox: Mailbox::new(mbx_cpu_irq_tx, mbx_cop_irq_tx),
            dmacon0,
            dmacon1,
            serial0: Serial::new("0"),
            serial1: Serial::new("1"),
            evp: Evp::new(),
            pwmcon: PWMCon::new(),

            mystery_irq_con: Stub::new("Mystery IRQ Con?"),
            mystery_flash_stub: Stub::new("Mystery FlashROM Con?"),
            total_mystery: Stub::new("(?) Arbiter Priority"),
            pp5002_serial_stub: Stub::new("PP5002 serial stub"),
        }
    }

    /// Reset the peripherals a warm reset touches.
    pub fn warm_reset(&mut self) {
        self.memcon.reset();
        self.cachecon.reset();
        self.evp.reset();
        self.cpucon.reset();
        self.intcon.reset();
        self.devcon.reset();
    }
}

macro_rules! soc_mmap {
    ($($start:literal $(..= $end:literal)? => $dev:ident,)*) => {
        macro_rules! impl_mem_r {
            ($fn:ident, $ret:ty) => {
                fn $fn(&mut self, addr: u32) -> MemResult<$ret> {
                    match addr {
                        $($start$(..=$end)? => self.$dev.$fn(addr - $start),)*
                        _ => Err(Unexpected),
                    }
                }
            };
        }

        macro_rules! impl_mem_w {
            ($fn:ident, $val:ty) => {
                fn $fn(&mut self, addr: u32, val: $val) -> MemResult<()> {
                    match addr {
                        $($start$(..=$end)? => self.$dev.$fn(addr - $start, val),)*
                        _ => Err(Unexpected),
                    }
                }
            };
        }

        impl Device for Pp502x {
            fn kind(&self) -> &'static str {
                "PP502x"
            }

            fn probe(&self, addr: u32) -> Probe {
                match addr {
                    $($start$(..=$end)? => Probe::from_device(&self.$dev, addr - $start),)*
                    _ => Probe::Unmapped,
                }
            }
        }

        impl Memory for Pp502x {
            impl_mem_r!(r8, u8);
            impl_mem_r!(r16, u16);
            impl_mem_r!(r32, u32);
            impl_mem_w!(w8, u8);
            impl_mem_w!(w16, u16);
            impl_mem_w!(w32, u32);
            impl_mem_r!(x16, u16);
            impl_mem_r!(x32, u32);
        }
    };
}

soc_mmap! {
    0x4000_0000..=0x4001_7fff => fastram,

    0x6000_0000..=0x6000_0fff => cpuid,
    0x6000_1000..=0x6000_102f => mailbox,
    0x6000_4000..=0x6000_41ff => intcon,
    0x6000_5000..=0x6000_5007 => timer1,
    0x6000_5008..=0x6000_500f => timer2,
    0x6000_5010..=0x6000_5013 => usec_timer,
    0x6000_5014..=0x6000_5017 => rtc,
    0x6000_6000..=0x6000_6fff => devcon,
    0x6000_7000..=0x6000_7fff => cpucon,
    // Memory accesses to dmacon1 are suspiciously similar to dmacon0
    0x6000_8000..=0x6000_9fff => dmacon1,
    0x6000_a000..=0x6000_bfff => dmacon0,
    0x6000_c000..=0x6000_cfff => cachecon,
    0x6000_d000..=0x6000_d07f => gpio_abcd,
    0x6000_d080..=0x6000_d0ff => gpio_efgh,
    0x6000_d100..=0x6000_d17f => gpio_ijkl,
    0x6000_d800..=0x6000_d87f => gpio_mirror_abcd,
    0x6000_d880..=0x6000_d8ff => gpio_mirror_efgh,
    0x6000_d900..=0x6000_d97f => gpio_mirror_ijkl,

    0x6400_4000..=0x6400_41ff => intcon, // i guess there's a mirror?

    0x7000_0000..=0x7000_1fff => ppcon,
    0x7000_3000..=0x7000_301f => mlcd,
    0x7000_6000..=0x7000_603f => serial0,
    0x7000_6040..=0x7000_607f => serial1,
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

    // Four registers at +0x00/+0x04/+0x08/+0x0c, RetailOS programs them
    // from one straight-line routine, never touched in diags.
    //
    // The values it writes are a cyclic Latin square:
    //
    //           +0x00  +0x04  +0x08  +0x0c
    //   bits0-1   0      1      2      3
    //   bits2-3   3      0      1      2
    //   bits4-5   2      3      0      1
    //   bits6-7   1      2      3      0
    //
    // Undocumented everywhere. Arbiter priority matrix of Multi Path Mem
    // Controller?
    0x6000_3000..=0x6000_30ff => total_mystery,
    // Diagnostics program reads from address, and write back 0x10000000
    0x7000_3800 => total_mystery,
    0xc031_b1d8 => mystery_flash_stub,
    0xc031_b1e8 => mystery_flash_stub,
    0xc500_0000..=0xc500_01ff => usb,
    0xc600_0000..=0xc600_01ff => firewire,
    0xffff_fe00..=0xffff_ffff => mystery_flash_stub,

    // PP5002 addresses, I know, but iPodLinux uses that
    0xc000_6000..=0xc000_6020 => pp5002_serial_stub,
    0xc000_6040..=0xc000_6060 => pp5002_serial_stub,
}
