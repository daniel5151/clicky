#![allow(non_camel_case_types)]

use crate::devices::prelude::*;

use bitfields::{bitfield, bitflag};

#[bitflag(u8)]
#[derive(Debug, PartialEq)]
enum RequestSelection {
    #[base]
    REQ_0 = 0,
    REQ_1 = 1,
    IIS = 2,
    REQ_3 = 3,
    REQ_4 = 4,
    REQ_5 = 5,
    REQ_6 = 6,
    REQ_7 = 7,
    REQ_8 = 8,
    REQ_9 = 9,
    REQ_10 = 10,
    LCD2 = 11,
    REQ_12 = 12,
    SDHC = 13,
    REQ_14 = 14,
    REQ_15 = 15,
}

#[bitflag(u8)]
#[derive(Debug, PartialEq)]
enum Direction {
    #[base]
    ApbToAhb = 0,
    AhbToApb = 1,
}

// Tegra 2 TRM 10.5.4.1 "AHBDMACHAN_CHANNEL_0_CSR_0"
#[bitfield(u32, order = msb)]
struct ChannelControl {
    enb: bool,

    /// Interrupt Enable on transfer complete
    ie_eoc: bool,

    /// Flush, doesn't stick
    #[bits(access = ro)]
    fl_btwn: bool,

    /// Doesn't stick
    #[bits(access = ro)]
    hold: bool,

    #[bits(1)]
    dir: Direction,

    /// 0 = Multiple Block Transfer
    once: bool,

    trig: bool,

    /// Only move data while the request line is asserted
    flow: bool,

    /// Trigger sources are unknown on PP5020
    #[bits(4)]
    trig_sel: u8,

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

    /// End of transfer interrupt, cleared by reading the register
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

/// Reserved encodings transfer 8 words
#[bitflag(u8)]
#[derive(Debug, PartialEq)]
enum AhbBurst {
    #[base]
    RSVD_0 = 0,
    RSVD_1 = 1,
    RSVD_2 = 2,
    RSVD_3 = 3,
    DMA_BURST_1WORDS = 4,
    DMA_BURST_4WORDS = 5,
    DMA_BURST_8WORDS = 6,
    RSVD_7 = 7,
}

#[bitflag(u8)]
#[derive(Debug, PartialEq)]
enum AhbWrap {
    #[base]
    NO_WRAP = 0,
    WRAP_ON_32WORDS = 1,
    WRAP_ON_64WORDS = 2,
    WRAP_ON_128WORDS = 3,
    WRAP_ON_256WORDS = 4,
    WRAP_ON_512WORDS = 5,
    WRAP_ON_1024WORDS = 6,
    WRAP_ON_2048WORDS = 7,
}

// Tegra 2 TRM 11.2.4.4 "APBDMACHAN_CHANNEL_0_AHB_SEQ_0"
#[bitfield(u32, order = msb)]
struct AhbSequencer {
    /// INTR_ENB, AHB_BUS_WIDTH and AHB_DATA_SWAP on Tegra, none of them stick
    #[bits(5)]
    _reserved_27: u8,

    #[bits(3)]
    ahb_burst: AhbBurst,

    #[bits(4)]
    _reserved_20: u8,

    /// Reload base address every other block
    dbl_buf: bool,

    #[bits(3)]
    ahb_addr_wrap: AhbWrap,

    _reserved_0: u16,
}

#[bitflag(u8)]
#[derive(Debug, PartialEq)]
enum BusWidth {
    #[base]
    BUS_WIDTH_8 = 0,
    BUS_WIDTH_16 = 1,
    BUS_WIDTH_32 = 2,
    BUS_WIDTH_64 = 3,
    BUS_WIDTH_128 = 4,
    RSVD_5 = 5,
    RSVD_6 = 6,
    RSVD_7 = 7,
}

#[bitflag(u8)]
#[derive(Debug, PartialEq)]
enum ApbWrap {
    #[base]
    NO_WRAP = 0,
    WRAP_ON_1WORDS = 1,
    WRAP_ON_2WORDS = 2,
    WRAP_ON_4WORDS = 3,
    WRAP_ON_8WORDS = 4,
    WRAP_ON_16WORDS = 5,
    WRAP_ON_32WORDS = 6,
    WRAP_ON_64WORDS = 7,
}

// Tegra 2 TRM 11.2.4.6 "APBDMACHAN_CHANNEL_0_APB_SEQ_0"
#[bitfield(u32, order = msb)]
struct ApbSequencer {
    _reserved_31: bool,

    #[bits(3, default = BusWidth::BUS_WIDTH_32)]
    apb_bus_width: BusWidth,

    #[bits(9)]
    _reserved_19: u16,

    #[bits(3, default = ApbWrap::WRAP_ON_1WORDS)]
    apb_addr_wrap: ApbWrap,

    _reserved_0: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct DmaTransfer {
    channel: usize,
    src_ptr: u32,
    dst_ptr: u32,
    width: u32,
}

#[derive(Debug, Default)]
struct DmaChannel {
    label: Option<&'static str>,

    control: ChannelControl,
    status: ChannelStatus,
    ahb_ptr: u32,
    ahb_seq: AhbSequencer,
    apb_ptr: u32,
    apb_seq: ApbSequencer,

    ahb_cursor: u32,
    apb_cursor: u32,
    remaining: u32,
    /// APB bus width
    width: u32,
}

impl DmaChannel {
    fn new(label: Option<&'static str>) -> DmaChannel {
        DmaChannel {
            label,

            control: ChannelControl::new(),
            status: ChannelStatus::new(),
            ahb_ptr: 0,
            ahb_seq: AhbSequencer::new(),
            apb_ptr: 0x7000_0000,
            apb_seq: ApbSequencer::new(),

            ahb_cursor: 0,
            apb_cursor: 0,
            remaining: 0,
            width: 4,
        }
    }

    fn latch(&mut self) {
        self.ahb_cursor = self.ahb_ptr;
        self.apb_cursor = self.apb_ptr;
        self.remaining = (self.control.wcount() as u32 + 1) * 4;
    }

    fn next_transfer(&self, channel: usize, requests: u16) -> Option<DmaTransfer> {
        if !self.control.enb() || self.remaining == 0 {
            return None;
        }

        if self.control.flow() && !requests.get_bit(self.control.req_sel() as usize) {
            return None;
        }

        let (src_ptr, dst_ptr) = match self.control.dir() {
            Direction::ApbToAhb => (self.apb_cursor, self.ahb_cursor),
            Direction::AhbToApb => (self.ahb_cursor, self.apb_cursor),
        };

        Some(DmaTransfer {
            channel,
            src_ptr,
            dst_ptr,
            width: self.width,
        })
    }
}

impl Device for DmaChannel {
    fn kind(&self) -> &'static str {
        "APB-DMA channel"
    }

    fn label(&self) -> Option<&'static str> {
        self.label
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "CSR",
            0x04 => "STA",
            0x08 | 0x0c => "APB_SEQ",
            0x10 => "AHB_PTR",
            0x14 => "AHB_SEQ",
            0x18 => "APB_PTR",
            0x1c => "APB_SEQ",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for DmaChannel {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => Ok(self.control.into_bits()),
            0x04 => {
                let mut status = self.status;
                status.set_bsy(self.control.enb());
                status.set_count((self.remaining / 4) as u16);
                self.status.set_is_eoc(false);
                Ok(status.into_bits())
            }
            0x08 | 0x0c | 0x1c => Ok(self.apb_seq.into_bits()),
            0x10 => Ok(self.ahb_ptr),
            0x14 => Ok(self.ahb_seq.into_bits()),
            0x18 => Ok(self.apb_ptr),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => {
                let was_enabled = self.control.enb();
                self.control.write_bits(val);
                if !self.control.enb() || was_enabled {
                    return Ok(());
                }

                if self.ahb_seq.dbl_buf() {
                    return Err(Unimplemented);
                }
                self.width = match self.apb_seq.apb_bus_width() {
                    BusWidth::BUS_WIDTH_8 => 1,
                    BusWidth::BUS_WIDTH_16 => 2,
                    BusWidth::BUS_WIDTH_32 => 4,
                    other => return Err(Fatal(format!("unsupported APB bus width {:?}", other))),
                };
                self.latch();
                debug!(
                    target: "DMA",
                    "APB-DMA:{} start csr={:#010x} ahb={:#010x} apb={:#010x} apb_seq={:#010x}",
                    self.label.unwrap_or("?"),
                    val,
                    self.ahb_ptr,
                    self.apb_ptr,
                    self.apb_seq.into_bits()
                );
                Ok(())
            }
            0x04 => Ok(()),
            0x08 | 0x0c | 0x1c => Ok(self.apb_seq.write_bits(val)),
            0x10 => Ok(self.ahb_ptr = val & !0b11),
            0x14 => Ok(self.ahb_seq.write_bits(val)),
            0x18 => Ok(self.apb_ptr = (val & 0xfffc) | 0x7000_0000),
            _ => Err(Unexpected),
        }
    }
}

// Tegra 2 TRM 10.5.3.1 "AHBDMA_CMD_0"
#[bitfield(u32, order = msb)]
struct CommandRegister {
    /// Global enable
    gen_en: bool,

    #[bits(31)]
    _reserved: u32,
}

// Tegra 2 TRM 10.5.3.2 "AHBDMA_STA_0"
#[bitfield(u32, order = msb)]
struct StatusRegister {
    #[bits(4)]
    _reserved_28: u8,

    ch3_busy: bool,
    ch2_busy: bool,
    ch1_busy: bool,
    ch0_busy: bool,

    #[bits(24)]
    _reserved_0: u32,
}

#[bitfield(u32, order = msb)]
struct RequestStatus {
    _reserved: u16,

    req_15: bool,
    req_14: bool,
    sdhc: bool,
    req_12: bool,
    #[bits(default = true)]
    lcd2: bool,
    req_10: bool,
    req_9: bool,
    req_8: bool,
    req_7: bool,
    req_6: bool,
    req_5: bool,
    #[bits(default = true)]
    req_4: bool,
    #[bits(default = true)]
    req_3: bool,
    #[bits(default = true)]
    iis: bool,
    req_1: bool,
    req_0: bool,
}

#[derive(Debug)]
pub struct ApbDma {
    irq: irq::Sender,

    dma: [DmaChannel; 4],
    command: CommandRegister,
    status: StatusRegister,
    req_status: RequestStatus,
}

impl ApbDma {
    pub fn new(irq: irq::Sender) -> ApbDma {
        ApbDma {
            irq,

            dma: [
                DmaChannel::new(Some("0")),
                DmaChannel::new(Some("1")),
                DmaChannel::new(Some("2")),
                DmaChannel::new(Some("3")),
            ],
            command: CommandRegister::new(),
            status: StatusRegister::new(),
            req_status: RequestStatus::new(),
        }
    }

    /// Get next DMA read/write request to SoC bus
    pub fn next_transfer(&self) -> Option<DmaTransfer> {
        let requests = self.req_status.into_bits() as u16;
        self.dma
            .iter()
            .enumerate()
            .find_map(|(channel, ch)| ch.next_transfer(channel, requests))
    }
}

impl Device for ApbDma {
    fn kind(&self) -> &'static str {
        "APB-DMA"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x0000..=0x0fff => match offset % 0x10 {
                0x0 => "CMD",
                0x4 => "STA",
                0x8 => "REQ_STATUS",
                _ => return Probe::Unmapped,
            },
            0x1000..=0x1fff => {
                let offset = (offset - 0x1000) % 0x80;
                return Probe::from_device(&self.dma[(offset / 0x20) as usize], offset % 0x20);
            }
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for ApbDma {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0000..=0x0fff => match offset % 0x10 {
                0x0 => Ok(self.command.into_bits()),
                0x4 => Ok(self.status.into_bits()),
                0x8 => Ok(self.req_status.into_bits()),
                _ => Err(Unexpected),
            },
            0x1000..=0x1fff => {
                let offset = (offset - 0x1000) % 0x80;
                self.dma[(offset / 0x20) as usize].r32(offset % 0x20)
            }
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x0000..=0x0fff => match offset % 0x10 {
                0x0 => Ok(self.command.write_bits(val)),
                0x4 | 0x8 => Err(InvalidAccess),
                _ => Err(Unexpected),
            },
            0x1000..=0x1fff => {
                let offset = (offset - 0x1000) % 0x80;
                self.dma[(offset / 0x20) as usize].w32(offset % 0x20, val)
            }
            _ => Err(Unexpected),
        }
    }
}
