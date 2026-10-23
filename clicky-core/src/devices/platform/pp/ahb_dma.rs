#![allow(non_camel_case_types)]

use crate::devices::prelude::*;

use bitfields::{bitfield, bitflag};

#[bitflag(u8)]
#[derive(Debug, PartialEq)]
enum TriggerSelection {
    #[base]
    SMP_30_same_as_bit_14 = 0,
    SMP_31_same_as_bit_15 = 1,
    HRQ2_XRQ_C = 2,
    HRQ3_XRQ_D = 3,
    HRQ4_N_A = 4,
    HRQ5_N_A = 5,
    HRQ6_TMR1 = 6,
    HRQ7_TMR2 = 7,
    AHB_0 = 8,
    AHB_1 = 9,
    AHB_2 = 10,
    AHB_3 = 11,
    SMP_28 = 12,
    SMP_29 = 13,
    SMP_30 = 14,
    SMP_31 = 15,
}

#[bitflag(u8)]
#[derive(Debug, PartialEq)]
enum RequestSelection {
    #[base]
    CNTR_REQ = 0,
    SMP_17 = 1,
    SMP_18 = 2,
    SMP_19 = 3,
    SMP_20 = 4,
    SMP_21 = 5,
    SMP_22 = 6,
    SMP_23 = 7,
    SMP_24 = 8,
    SMP_25 = 9,
    SMP_26 = 10,
    Host1x = 11,
    SRQ0_N_A = 12,
    SRQ1_N_A = 13,
    SRQ0_XRQ_A = 14,
    SRQ1_XRQ_B = 15,
}

// Tegra 2 TRM 10.5.4.1 "AHBDMACHAN_CHANNEL_0_CSR_0"
#[bitfield(u32, order = msb)]
struct ChannelControl {
    enb: bool,

    /// Interrupt Enable on transfer complete
    ie_eoc: bool,

    /// Flush
    fl_btwn: bool,

    hold: bool,

    /// 0 = XMB -> AHB
    dir: bool,

    /// 0 = Multiple Block Transfer
    once: bool,

    trig: bool,

    flow: bool,

    #[bits(4)]
    trig_sel: TriggerSelection,

    #[bits(4)]
    req_sel: RequestSelection,

    /// Number of words to transfer, minus one
    #[bits(14)]
    wcount: u16,

    #[bits(2)]
    _reserved: u8,
}

// Tegra 2 TRM 10.5.4.2 "AHBDMACHAN_CHANNEL_0_STA_0"
#[bitfield(u32, order = msb)]
struct ChannelStatus {
    bsy: bool,

    /// End of transfer interrupt, write 1 to clear
    is_eoc: bool,

    _reserved_29: bool,

    halt: bool,

    #[bits(12)]
    _reserved_16: u16,

    /// Remaining words
    #[bits(14)]
    count: u16,

    #[bits(2)]
    _reserved_0: u8,
}

#[bitflag(u8)]
#[derive(Debug, PartialEq)]
enum StrideSize {
    #[base]
    DISABLE = 0,
    WORDS_4 = 1,
    WORDS_8 = 2,
    WORDS_16 = 3,
    WORDS_32 = 4,
    WORDS_64 = 5,
    WORDS_128 = 6,
    WORDS_256 = 7,
}

#[bitflag(u8)]
#[derive(Debug, PartialEq)]
enum AhbBurst {
    #[base]
    RSVD_0 = 0,
    RSVD_1 = 1,
    WORD = 2,
    FOUR_WORD = 3,
    EIGHT_WORD = 4,
    RSVD_5 = 5,
    RSVD_6 = 6,
    RSVD_7 = 7,
}

// Tegra 2 TRM 10.5.4.4 "AHBDMACHAN_CHANNEL_0_AHB_SEQ_0"
#[bitfield(u32, order = msb)]
struct AhbAddressSequencer {
    /// Send interrupt to CPU (0 = send to COP)
    intr_enb: bool,

    #[bits(3)]
    _reserved_28: u8,

    /// Reverse endianness
    ahb_data_swap: bool,

    #[bits(3, default = AhbBurst::WORD)]
    ahb_burst: AhbBurst,

    #[bits(3)]
    ahb_stride_size: StrideSize,

    _reserved_20: bool,

    /// Reload base address every time
    dbl_buf: bool,

    ahb_addr_wrap: bool,

    /// Words skipped between strides
    ahb_stride: u16,

    #[bits(2)]
    _reserved_0: u8,
}

// Tegra 2 TRM 10.5.4.6 "AHBDMACHAN_CHANNEL_0_XMB_SEQ_0"
#[bitfield(u32, order = msb)]
struct XmbAddressSequencer {
    #[bits(4)]
    _reserved_28: u8,

    /// Reverse endianness
    xmb_data_swap: bool,

    #[bits(7)]
    _reserved_20: u8,

    /// Reload base address every time
    dbl_buf: bool,

    xmb_addr_wrap: bool,

    /// Words skipped between strides
    xmb_stride: u16,

    #[bits(2)]
    _reserved_0: u8,
}

#[derive(Debug, Default)]
struct DmaChannel {
    label: Option<&'static str>,

    control: ChannelControl,
    status: ChannelStatus,
    ahb_ptr: u32,
    ahb_seq: AhbAddressSequencer,
    xmb_ptr: u32,
    xmb_seq: XmbAddressSequencer,
}

impl DmaChannel {
    fn new(label: Option<&'static str>) -> DmaChannel {
        DmaChannel {
            label,

            control: ChannelControl::new(),
            status: ChannelStatus::new(),
            ahb_ptr: 0,
            ahb_seq: AhbAddressSequencer::new(),
            xmb_ptr: 0,
            xmb_seq: XmbAddressSequencer::new(),
        }
    }
}

impl Device for DmaChannel {
    fn kind(&self) -> &'static str {
        "AHB-DMA channel"
    }

    fn label(&self) -> Option<&'static str> {
        self.label
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "CSR",
            0x04 => "STA",
            0x10 => "AHB_PTR",
            0x14 => "AHB_SEQ",
            0x18 => "XMB_PTR",
            0x1c => "XMB_SEQ",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for DmaChannel {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => Ok(self.control.into_bits()),
            0x04 => Ok(self.status.into_bits()),
            0x10 => Ok(self.ahb_ptr),
            0x14 => Ok(self.ahb_seq.into_bits()),
            0x18 => Ok(self.xmb_ptr),
            0x1c => Ok(self.xmb_seq.into_bits()),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => {
                self.control.write_bits(val);
                if self.control.enb() {
                    return Err(StubWrite(Warn, ()));
                }
                Ok(())
            }
            0x04 => {
                if ChannelStatus::from_bits(val).is_eoc() {
                    self.status.set_is_eoc(false);
                }
                Ok(())
            }
            0x10 => Ok(self.ahb_ptr = val),
            0x14 => Ok(self.ahb_seq.write_bits(val)),
            0x18 => Ok(self.xmb_ptr = val),
            0x1c => Ok(self.xmb_seq.write_bits(val)),
            _ => Err(Unexpected),
        }
    }
}

// Tegra 2 TRM 10.5.3.1 "AHBDMA_CMD_0"
#[bitfield(u32, order = msb)]
struct CommandRegister {
    /// Global enable
    gen: bool,

    #[bits(31)]
    _reserved: u32,
}

// Tegra 2 TRM 10.5.3.2 "AHBDMA_STA_0"
#[bitfield(u32, order = msb)]
struct StatusRegister {
    #[bits(6)]
    _reserved_26: u8,

    ch1_busy: bool,
    ch0_busy: bool,

    #[bits(24)]
    _reserved_0: u32,
}

// Tegra 2 TRM 10.5.3.3 "AHBDMA_TX_REQ_0"
#[bitfield(u32, order = msb)]
struct TxRequestors {
    smp_31: bool,
    smp_30: bool,
    smp_29: bool,
    smp_28: bool,

    #[bits(2)]
    _reserved_26: u8,

    /// End of transfer on channel 1
    ahb_1: bool,
    /// End of transfer on channel 0
    ahb_0: bool,

    hrq7_tmr2: bool,
    hrq6_tmr1: bool,

    #[bits(2)]
    _reserved_20: u8,

    hrq3_xrq_d: bool,
    hrq2_xrq_c: bool,
    smp_31_same_as_31_bit: bool,
    smp_30_same_as_30_bit: bool,
    srq1_xrq_b: bool,
    srq0_xrq_a: bool,

    #[bits(2)]
    _reserved_12: u8,

    host1x: bool,
    smp_26: bool,
    smp_25: bool,
    smp_24: bool,
    smp_23: bool,
    smp_22: bool,
    smp_21: bool,
    smp_20: bool,
    smp_19: bool,
    smp_18: bool,
    smp_17: bool,
    cntr_req: bool,
}

// Tegra 2 TRM 10.5.3.4 "AHBDMA_COUNTER_0"
#[bitfield(u32, order = msb)]
struct CounterUsage {
    #[bits(12)]
    _reserved: u16,

    /// Throttle channel 1 on the counter
    ch1_fl_cnt: bool,
    /// Throttle channel 0 on the counter
    ch0_fl_cnt: bool,

    cntr_en: bool,

    /// Qualify counter reload with the last word of a burst
    prd_en: bool,

    /// Counter init/reload value
    count_value: u16,
}

// Tegra 2 TRM 10.5.3.5 "AHBDMA_IRQ_STA_CPU_0", 10.5.3.6 "AHBDMA_IRQ_STA_COP_0"
#[bitfield(u32, order = msb)]
struct IrqStatus {
    #[bits(30)]
    _reserved: u32,

    ch1: bool,
    ch0: bool,
}

// Tegra 2 TRM 10.5.3.7 "AHBDMA_IRQ_MASK_0", 10.5.3.8 "AHBDMA_IRQ_MASK_SET_0",
// 10.5.3.9 "AHBDMA_IRQ_MASK_CLR_0"
#[bitfield(u32, order = msb)]
struct IrqMask {
    #[bits(30)]
    _reserved: u32,

    ch1: bool,
    ch0: bool,
}

// Tegra 2 TRM 10.5.3.10 "AHBDMA_RDWR_COHERENCY_0"
#[bitfield(u32, order = msb)]
struct RdWrCoherency {
    /// No AHB writes in flight
    #[bits(default = true)]
    mst_gizmo_wrque_empty: bool,

    #[bits(13)]
    _reserved_18: u16,

    ch1_rds_active: bool,
    ch0_rds_active: bool,

    #[bits(14)]
    _reserved_2: u16,

    ch1_wrs_active: bool,
    ch0_wrs_active: bool,
}

// Tegra 2 TRM 10.5.3.11 "AHBDMA_TEST_BUS_0"
#[bitfield(u32, order = msb)]
struct TestBus {
    #[bits(28)]
    _reserved: u32,

    mstgizmo_wrque_ch1_pong_status: bool,
    mstgizmo_wrque_ch1_ping_status: bool,
    mstgizmo_wrque_ch0_pong_status: bool,
    mstgizmo_wrque_ch0_ping_status: bool,
}

#[derive(Debug)]
pub struct AhbDma {
    dma: [DmaChannel; 2],
    command: CommandRegister,
    status: StatusRegister,
    tx_req: TxRequestors,
    counter: CounterUsage,
    irq_mask: IrqMask,
    rdwr_coherency: RdWrCoherency,
    test_bus: TestBus,
}

impl AhbDma {
    pub fn new() -> AhbDma {
        AhbDma {
            dma: [DmaChannel::new(Some("0")), DmaChannel::new(Some("1"))],
            command: CommandRegister::new(),
            status: StatusRegister::new(),
            tx_req: TxRequestors::new(),
            counter: CounterUsage::new(),
            irq_mask: IrqMask::new(),
            rdwr_coherency: RdWrCoherency::new(),
            test_bus: TestBus::new(),
        }
    }

    /// Masked end-of-transfer interrupts routed to the CPU (or the COP).
    fn irq_status(&self, cpu: bool) -> IrqStatus {
        let pending = |ch: &DmaChannel, unmasked| {
            unmasked && ch.status.is_eoc() && ch.ahb_seq.intr_enb() == cpu
        };

        let mut status = IrqStatus::new();
        status.set_ch0(pending(&self.dma[0], self.irq_mask.ch0()));
        status.set_ch1(pending(&self.dma[1], self.irq_mask.ch1()));
        status
    }
}

impl Device for AhbDma {
    fn kind(&self) -> &'static str {
        "AHB-DMA"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "AHBDMA_CMD",
            0x04 => "AHBDMA_STA",
            0x08 => "AHBDMA_TX_REQ",
            0x10 => "AHBDMA_COUNTER",
            0x14 => "AHBDMA_IRQ_STA_CPU",
            0x18 => "AHBDMA_IRQ_STA_COP",
            0x1c => "AHBDMA_IRQ_MASK",
            0x20 => "AHBDMA_IRQ_MASK_SET",
            0x24 => "AHBDMA_IRQ_MASK_CLR",
            0x28 => "AHBDMA_RDWR_COHERENCY",
            0x2c => "AHBDMA_TEST_BUS",
            0x1000..=0x1fff => match (offset - 0x1000) % 0x80 {
                offset @ 0x00..=0x3f => {
                    return Probe::from_device(&self.dma[(offset / 0x20) as usize], offset % 0x20);
                }
                _ => "Reserved (reads zero)",
            },
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for AhbDma {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => Ok(self.command.into_bits()),
            0x04 => Ok(self.status.into_bits()),
            0x08 => Ok(self.tx_req.into_bits()),
            0x10 => Ok(self.counter.into_bits()),
            0x14 => Ok(self.irq_status(true).into_bits()),
            0x18 => Ok(self.irq_status(false).into_bits()),
            0x1c => Ok(self.irq_mask.into_bits()),
            0x20 | 0x24 => Err(InvalidAccess),
            0x28 => Ok(self.rdwr_coherency.into_bits()),
            0x2c => Ok(self.test_bus.into_bits()),
            0x1000..=0x1fff => match (offset - 0x1000) % 0x80 {
                offset @ 0x00..=0x3f => self.dma[(offset / 0x20) as usize].r32(offset % 0x20),
                _ => Ok(0),
            },
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => Ok(self.command.write_bits(val)),
            0x04 | 0x08 | 0x14 | 0x18 | 0x1c | 0x28 | 0x2c => Err(InvalidAccess),
            0x10 => Ok(self.counter.write_bits(val)),
            0x20 => {
                let set = IrqMask::from_bits(val);
                self.irq_mask.set_ch0(self.irq_mask.ch0() || set.ch0());
                self.irq_mask.set_ch1(self.irq_mask.ch1() || set.ch1());
                Ok(())
            }
            0x24 => {
                let clr = IrqMask::from_bits(val);
                self.irq_mask.set_ch0(self.irq_mask.ch0() && !clr.ch0());
                self.irq_mask.set_ch1(self.irq_mask.ch1() && !clr.ch1());
                Ok(())
            }
            0x1000..=0x1fff => match (offset - 0x1000) % 0x80 {
                offset @ 0x00..=0x3f => self.dma[(offset / 0x20) as usize].w32(offset % 0x20, val),
                _ => Ok(()),
            },
            _ => Err(Unexpected),
        }
    }
}
