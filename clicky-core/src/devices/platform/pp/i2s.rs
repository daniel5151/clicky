#![allow(non_camel_case_types)]

use crate::devices::prelude::*;
use bitfields::{bitfield, bitflag};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use relativity::Instant;

use crate::gui::AudioCallback;

const OUTPUT_FIFO_SLOTS: usize = 16;
const SAMPLE_RATE: u32 = 44_100;

/// Most audio buffered for the host, in slots (0.25 s)
const HOST_BUFFER_SLOTS: usize = SAMPLE_RATE as usize / 4;

#[bitflag(u8)]
#[derive(Debug, PartialEq)]
enum BitFormat {
    #[base]
    I2S = 0,
    RJM = 1,
    LJM = 2,
    DSP = 3,
}

#[bitflag(u8)]
#[derive(Debug, PartialEq)]
enum BitSize {
    #[base]
    BIT_SIZE_16 = 0,
    BIT_SIZE_20 = 1,
    BIT_SIZE_24 = 2,
    BIT_SIZE_32 = 3,
}

#[bitflag(u8)]
#[derive(Debug, PartialEq)]
enum FifoFormat {
    #[base]
    BIT_SIZE_16_LSB = 0,
    BIT_SIZE_20_LSB = 1,
    BIT_SIZE_24_LSB = 2,
    BIT_SIZE_32 = 3,
    RSVD_4 = 4,
    RSVD_5 = 5,
    RSVD_6 = 6,
    /// 16-bit samples, L-R packed into a 32-bit slot with left in the LSBs
    PACKED = 7,
}

// Tegra 2 TRM 27.2.6.1 "I2S_I2S_CTRL_0"
#[bitfield(u32, order = msb)]
struct Control {
    /// Not in TRM, seen in Rockbox
    reset: bool,

    tx2_enable: bool,

    fifo1_transaction_en: bool,

    fifo2_transaction_en: bool,

    rx2_enable: bool,

    /// Tx1-Rx1 loop back
    lpbk: bool,

    /// Master
    m_s: bool,

    /// Left channel when LRCK is high
    l_r: bool,

    #[bits(12)]
    _reserved_12: u16,

    #[bits(2)]
    bit_format: BitFormat,

    #[bits(2)]
    bit_size: BitSize,

    _reserved_7: bool,

    #[bits(3)]
    fifo_format: FifoFormat,

    /// Interrupt on FIFO1 underrun (Tx) / overrun (Rx)
    ie_fifo1_err: bool,

    /// Interrupt on FIFO2 underrun (Tx) / overrun (Rx)
    ie_fifo2_err: bool,

    /// Interrupt on FIFO1 trigger
    qe_fifo1: bool,

    /// Interrupt on FIFO2 trigger
    qe_fifo2: bool,
}

// Tegra 2 TRM 27.2.6.2 "I2S_I2S_STATUS_0"
#[bitfield(u32, order = msb)]
struct Status {
    /// FIFO1 transaction synchronized to LRCK
    fifo1_rdy: bool,

    /// FIFO2 transaction synchronized to LRCK
    fifo2_rdy: bool,

    /// FIFO1 shifter busy
    fifo1_bsy: bool,

    /// FIFO2 shifter busy
    fifo2_bsy: bool,

    #[bits(24)]
    _reserved_4: u32,

    is_fifo1_err: bool,

    is_fifo2_err: bool,

    /// FIFO1 attention level reached
    qs_fifo1: bool,

    /// FIFO2 attention level reached
    qs_fifo2: bool,
}

// Tegra 2 TRM 27.2.6.3 "I2S_I2S_TIMING_0"
#[bitfield(u32, order = msb)]
struct Timing {
    #[bits(19)]
    _reserved_13: u32,

    /// Right channel gets one more bit clock than the left one
    non_sym_en: bool,

    _reserved_11: bool,

    /// Bit clocks per channel (or per L+R in DSP mode)
    #[bits(11, default = 0x1f)]
    channel_bit_cnt: u16,
}

// Tegra 2 TRM 27.2.6.4 "I2S_I2S_FIFO_SCR_0"
#[bitfield(u32, order = msb)]
struct FifoScr {
    #[bits(2)]
    _reserved_30: u8,

    /// Full (Rx) or empty (Tx) slots in FIFO2
    #[bits(6, access = ro)]
    fifo2_full_empty_count: u8,

    #[bits(2)]
    _reserved_22: u8,

    /// Full (Rx) or empty (Tx) slots in FIFO1
    #[bits(6, access = ro)]
    fifo1_full_empty_count: u8,

    #[bits(3)]
    _reserved_13: u8,

    /// Cleared by hardware
    fifo2_clr: bool,

    #[bits(3)]
    _reserved_9: u8,

    /// Cleared by hardware
    fifo1_clr: bool,

    #[bits(2)]
    _reserved_6: u8,

    #[bits(2)]
    fifo2_atn_lvl: u8,

    #[bits(2)]
    _reserved_2: u8,

    #[bits(2)]
    fifo1_atn_lvl: u8,
}

/// PP5020 I2S controller.
#[derive(Debug)]
pub struct I2SCon {
    drq: drq::Sender,

    control: Control,
    timing: Timing,
    fifo_scr: FifoScr,

    output_fifo: VecDeque<u32>,

    /// Last drain to host_buffer
    last_drain: Instant,
    host_buffer: Arc<Mutex<VecDeque<u32>>>,
}

impl I2SCon {
    pub fn new(drq: drq::Sender) -> I2SCon {
        let mut i2s = I2SCon {
            drq,

            control: Control::new(),
            timing: Timing::new(),
            fifo_scr: FifoScr::new(),
            output_fifo: VecDeque::with_capacity(OUTPUT_FIFO_SLOTS),
            last_drain: Instant::now(),
            host_buffer: Default::default(),
        };
        // the output FIFO starts empty
        i2s.update_drq();
        i2s
    }

    pub fn tick(&mut self) {
        if !self.control.fifo1_transaction_en() || self.output_fifo.is_empty() {
            return;
        }

        // Compute how much frame should be read from self.output_fifo to keep
        // a steady 44.1kHz output rate
        let now = Instant::now();
        let due_frames = (now.duration_since(self.last_drain).as_nanos() * SAMPLE_RATE as u128
            / 1_000_000_000) as usize;
        if due_frames == 0 {
            return;
        }

        if due_frames > self.output_fifo.len() {
            // Underrun (or first slots after idling): restart the clock from now
            self.last_drain = now;
        } else {
            self.last_drain +=
                Duration::from_nanos((due_frames as u64 * 1_000_000_000) / SAMPLE_RATE as u64);
        }

        // Move self.output_fifo frames into host_buffer
        let n = due_frames.min(self.output_fifo.len());
        let mut host_buffer = self.host_buffer.lock().unwrap();
        for slot in self.output_fifo.drain(..n) {
            // Drop slots when the host doesn't keep up
            if host_buffer.len() < HOST_BUFFER_SLOTS {
                host_buffer.push_back(slot);
            }
        }
        drop(host_buffer);

        self.update_drq();
    }

    /// Returns a callback playing the shifted out slots, mixed down to mono.
    pub fn audio_callback(&self) -> AudioCallback {
        let host_buffer = Arc::clone(&self.host_buffer);
        Box::new(move |out: &mut [f32], _sample_rate: u32| {
            let mut host_buffer = host_buffer.lock().unwrap();
            for sample in out.iter_mut() {
                *sample = match host_buffer.pop_front() {
                    // two 16-bit samples packed into a slot
                    Some(slot) => (slot as i16 as f32 + (slot >> 16) as i16 as f32) / 65536.0,
                    None => 0.0,
                };
            }
        })
    }

    fn update_drq(&mut self) {
        // For the audio output path, the attention level is the amount
        // of minimal free slots we want to use to trigger DMA.
        // Goal here is to AVOID STARVING.
        let attention_level = match self.fifo_scr.fifo1_atn_lvl() {
            0 => 1,
            1 => 4,
            2 => 8,
            3 => 12,
            _ => panic!("Unexpected attention level"),
        };
        let attention = OUTPUT_FIFO_SLOTS - self.output_fifo.len() >= attention_level;
        self.drq.set(attention);
    }
}

impl Device for I2SCon {
    fn kind(&self) -> &'static str {
        "I2S Controller"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "Control",
            0x04 => "Status",
            0x08 => "Timing",
            0x0c => "FIFO Control",
            0x40 => "Output FIFO",
            0x80 => "Input FIFO",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for I2SCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => Err(StubRead(Error, self.control.into_bits())),
            0x08 => Err(StubRead(Error, self.timing.into_bits())),
            0x0c => Err(StubRead(Error, self.fifo_scr.into_bits())),
            0x40 => Err(InvalidAccess),
            0x80 => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => {
                self.control.write_bits(val);
                if self.control.reset() {
                    self.output_fifo.clear();
                    self.update_drq();
                }

                Ok(())
            }
            0x08 => Err(StubWrite(Error, self.timing.write_bits(val))),
            0x0c => {
                // the attention level may have changed
                self.fifo_scr.write_bits(val);
                self.update_drq();
                Err(StubWrite(Error, ()))
            }
            0x40 => {
                if self.output_fifo.len() >= OUTPUT_FIFO_SLOTS {
                    return Err(ContractViolation {
                        msg: "write to full output FIFO, sample dropped".into(),
                        severity: Warn,
                        stub_val: None,
                    });
                }
                self.output_fifo.push_back(val);
                self.update_drq();
                Ok(())
            }
            0x80 => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }
}
