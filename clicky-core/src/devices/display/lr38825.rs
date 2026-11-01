// LR38825 resources:
// - https://www.mouser.com/ds/2/365/LQ022B8UD05_LCP-05032F-183713.pdf
// - https://github.com/Astralix/hardware_drivers/blob/6a6934a054551ac34968365491f1504a9cf3c685/dmw96/u-boot/u-boot/board/dspg/dw/lcd.c
// - https://github.com/Infineon/middleware-emwin/blob/master/Sample/DisplayDriver/CompactColor_16/GUIDRV_CompactColor_16.c
//
// LR38826 resources (similar to LR38825 in some aspects):
// - https://github.com/ua1arn/hftrx/blob/d2037da0fb57eac499de9e10cf75b8327bf1832b/boards/arm_rockchip_ctlstyle_opi3b.h
// - https://github.com/zhou88mang/hftrx/blob/1853c0fd11f113b64b8f6f9fe1d11b75191ec312/src/display/ls020.c
// - https://akost.github.io/siemens-s65-display/

use crate::devices::prelude::*;

use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::devices::display::{LcdController, LcdPanel};
use crate::devices::util::Deadline;
use crate::gui::RenderCallback;
use crate::signal::gpio;

use bitfields::{bitfield, bitflag};

const MAX_WIDTH: u8 = 220;
const MAX_HEIGHT: u8 = 176;
const GRAM_LEN: usize = MAX_WIDTH as usize * MAX_HEIGHT as usize;

// LCDINT frame pulse, measured on an iPod photo: 65.63 Hz, active-low.
const LCDINT_PERIOD: Duration = Duration::from_micros(15_237);
const LCDINT_LOW: Duration = Duration::from_micros(534);

fn rgb565_to_rgb888(p: u16) -> u32 {
    // RGB565 -> RGB888 conversion courtesy of
    // https://stackoverflow.com/questions/2442576/how-does-one-convert-16-bit-rgb565-to-24-bit-rgb888
    let r5 = p.get_bits(11..=15) as u32;
    let g6 = p.get_bits(5..=10) as u32;
    let b5 = p.get_bits(0..=4) as u32;

    let r8 = (r5 * 255) / 31;
    let g8 = (g6 * 255) / 63;
    let b8 = (b5 * 255) / 31;

    (r8 << 16) | (g8 << 8) | b8
}

/// Pixel data format from HIF1
#[bitflag(u8)]
#[derive(Debug, PartialEq, Eq)]
enum PixelFormat {
    /// 1BPP
    Rgb332 = 0b000,

    /// 2BPP
    #[base]
    Rgb565 = 0b001,

    /// 3BPP, dummy byte first
    PadFirstRgb565 = 0b010,

    /// 3BPP, dummy byte last
    PadLastRgb565 = 0b100,
}

/// HIF1 (0x10)
///
/// Most of these bit definitions were found through a lot of trial and
/// error on real hardware.
#[bitfield(u8)]
struct Hif1 {
    #[bits(2)]
    _unknown_0: u8,

    msb_first: bool,

    #[bits(3, default = PixelFormat::Rgb565)]
    format: PixelFormat,

    /// Swap red and blue
    bgr: bool,

    _unknown_7: bool,
}

/// HIF2 (0x11)
#[bitfield(u8)]
struct Hif2 {
    _unknown_0: bool,

    /// GRAM writes are dropped (or sent to a special place?)
    write_inhibit: bool,

    #[bits(6)]
    _unknown_2: u8,
}

/// AGM (0x18)
#[bitfield(u8)]
struct Agm {
    y_decr: bool,

    x_decr: bool,

    /// X address changes first when set, Y address when clear
    swap_xy: bool,

    /// GRAM writes are dropped when non-zero (either bit)
    #[bits(2)]
    write_inhibit: u8,

    #[bits(3)]
    _unknown_5: u8,
}

/// "Display displaying setting register" (sic) (0x7e)
#[bitfield(u8)]
struct DisplaySetting {
    /// Scan display from GRAM?
    /// LCDINT pulse keeps on running when cleared
    #[bits(default = true)]
    output: bool,

    #[bits(7)]
    _unknown_1: u8,
}

/// "Display displaying control register" (sic) (0x80)
#[bitfield(u8)]
struct DisplayControl {
    /// Scan display from GRAM?
    /// LCDINT pulse stops when cleared
    #[bits(default = true)]
    scan: bool,

    #[bits(7)]
    _unknown_1: u8,
}

#[derive(Debug, Default, Copy, Clone)]
struct InternalRegs {
    cmd: u16,

    hif1: Hif1,
    hif2: Hif2,
    agm: Agm,
    display_setting: DisplaySetting,
    display_control: DisplayControl,

    x_start: u8,
    x_end: u8,
    y_start: u8,
    y_end: u8,
    x_addr: u8,
    y_addr: u8,
}

pub struct LR38825 {
    panel: LcdPanel,
    gram: Arc<RwLock<[u16; GRAM_LEN]>>,
    ireg: InternalRegs,
    lcdint_gpio: Option<gpio::Sender>,
    lcdint_state: bool,
    lcdint_deadline: Deadline,
}

impl std::fmt::Debug for LR38825 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LR38825")
            .field("gram", &"[...]")
            .field("ireg", &self.ireg)
            .finish()
    }
}

impl LR38825 {
    pub fn new(panel: LcdPanel) -> LR38825 {
        LR38825 {
            panel,
            gram: Arc::new(RwLock::new([0; GRAM_LEN])),
            ireg: InternalRegs {
                x_end: MAX_WIDTH - 1,
                y_end: MAX_HEIGHT - 1,
                ..InternalRegs::default()
            },
            lcdint_gpio: None,
            lcdint_state: false,
            lcdint_deadline: Deadline::oneshot(Duration::ZERO),
        }
    }

    pub fn attach(&mut self, pin: Option<gpio::Sender>) {
        self.lcdint_gpio = pin;
    }

    fn write_ram(&mut self, val: u16) {
        // Perform memory write, check we're within bounds
        if self.ireg.x_addr < MAX_WIDTH && self.ireg.y_addr < MAX_HEIGHT {
            let x = self.ireg.x_addr as usize;
            let y = self.ireg.y_addr as usize;
            self.gram.write().unwrap()[y * MAX_WIDTH as usize + x] = val;
        }

        // Compute x and y address wraps
        let x_wrap = self.ireg.x_addr == self.ireg.x_end;
        let next_x = match (x_wrap, self.ireg.agm.x_decr()) {
            (true, _) => self.ireg.x_start,
            (false, true) => self.ireg.x_addr.wrapping_sub(1),
            (false, false) => self.ireg.x_addr.wrapping_add(1),
        };

        let y_wrap = self.ireg.y_addr == self.ireg.y_end;
        let next_y = match (y_wrap, self.ireg.agm.y_decr()) {
            (true, _) => self.ireg.y_start,
            (false, true) => self.ireg.y_addr.wrapping_sub(1),
            (false, false) => self.ireg.y_addr.wrapping_add(1),
        };

        // Update x and y addresses
        if self.ireg.agm.swap_xy() {
            self.ireg.x_addr = next_x;
            if x_wrap {
                self.ireg.y_addr = next_y;
            }
        } else {
            self.ireg.y_addr = next_y;
            if y_wrap {
                self.ireg.x_addr = next_x;
            }
        }
    }

    fn make_render_callback(&self) -> RenderCallback {
        let gram = Arc::clone(&self.gram);
        let panel = self.panel;
        Box::new(move |buf: &mut Vec<u32>| -> (usize, usize) {
            let gram = *gram.read().unwrap();
            let new_buf = gram.iter().map(|&px| 0xff000000 | rgb565_to_rgb888(px));

            buf.splice(.., new_buf);

            if panel.reverse_hor {
                for row in buf.chunks_exact_mut(MAX_WIDTH as usize) {
                    row.reverse()
                }
            }

            (MAX_WIDTH as usize, MAX_HEIGHT as usize)
        })
    }
}

impl LcdController for LR38825 {
    fn write_command(&mut self, val: u16) -> MemResult<()> {
        self.ireg.cmd = val >> 8;
        let val = val as u8;

        if let 0..=254 = self.ireg.cmd {
            trace!(target: "LCD", "write_command cmd:{:x} val:0x{:x}({})", self.ireg.cmd, val, val);
        }
        match self.ireg.cmd {
            // 0: Oscillator stop
            0x1 => {}

            // VMD
            0x3 => {}

            0x8 => {}

            0x10 => {
                self.ireg.hif1.write_bits(val);

                if self.ireg.hif1.format() != PixelFormat::Rgb565 {
                    error!(target: "LCD", "LR38825: unimplemented pixel format {:?}", self.ireg.hif1.format());
                    return Err(Unimplemented);
                }
                if self.ireg.hif1.bgr() {
                    error!(target: "LCD", "LR38825: unimplemented BGR pixel order");
                    return Err(Unimplemented);
                }
            }
            0x11 => {
                self.ireg.hif2.write_bits(val);
                if self.ireg.hif2.write_inhibit() {
                    error!(target: "LCD", "LR38825: unimplemented HIF2 write inhibit");
                    return Err(Unimplemented);
                }
            }

            // ASX
            0x12 => {
                self.ireg.y_start = val;
                self.ireg.y_addr = val;
            }

            // ASY
            0x13 => {
                self.ireg.x_start = val;
                self.ireg.x_addr = val;
            }

            // Horizontal RAM address
            0x14 => self.ireg.y_addr = val,

            // AEX
            0x15 => self.ireg.y_end = val,

            // AEY
            0x16 => self.ireg.x_end = val,

            // Vertical RAM address
            0x17 => self.ireg.x_addr = val,

            0x18 => {
                self.ireg.agm.write_bits(val);
                if self.ireg.agm.write_inhibit() != 0 {
                    error!(target: "LCD", "LR38825: unimplemented AGM write inhibit");
                    return Err(Unimplemented);
                }
            }
            0x7E => self.ireg.display_setting.write_bits(val),

            // "V sync parameter transfer flag"
            //  0: TG parameter refresh
            0x7F => {}

            0x80 => self.ireg.display_control.write_bits(val),

            // Display displaying color setting register
            0x88 => {}

            // Set Page
            // Page 0 = CPU bank
            0xef => {}

            // Unidentified commands used in RetailOS
            0x02 | 0xce => {}

            invalid_cmd => {
                return Err(Fatal(format!(
                    "attempted to execute invalid LCD command {:#x?}",
                    invalid_cmd
                )));
            }
        }
        Ok(())
    }

    fn read_command(&mut self) -> MemResult<u16> {
        Err(Unimplemented)
    }

    fn write_data(&mut self, val: u16) -> MemResult<()> {
        if self.ireg.hif1.msb_first() {
            self.write_ram(val.swap_bytes());
        } else {
            self.write_ram(val);
        }
        Ok(())
    }

    fn read_data(&mut self) -> MemResult<u16> {
        Err(Unimplemented)
    }

    fn render_callback(&self) -> RenderCallback {
        self.make_render_callback()
    }

    fn tick(&mut self) {
        if self.lcdint_gpio.is_none() || !self.lcdint_deadline.expired() {
            return;
        }

        let scan = self.ireg.display_control.scan();
        let gpio = self.lcdint_gpio.as_mut().unwrap();
        self.lcdint_state = !(self.lcdint_state && scan);
        if self.lcdint_state {
            gpio.set_high();
            self.lcdint_deadline = Deadline::oneshot(LCDINT_PERIOD - LCDINT_LOW);
        } else {
            gpio.set_low();
            self.lcdint_deadline = Deadline::oneshot(LCDINT_LOW);
        }
    }
}
