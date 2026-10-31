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
    panel: Option<LcdPanel>,
    gram: Arc<RwLock<[u16; GRAM_LEN]>>,
    ireg: Arc<RwLock<InternalRegs>>,
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
    pub fn new() -> LR38825 {
        LR38825 {
            panel: None,
            gram: Arc::new(RwLock::new([0; GRAM_LEN])),
            ireg: Arc::new(RwLock::new(InternalRegs {
                x_end: MAX_WIDTH - 1,
                y_end: MAX_HEIGHT - 1,
                ..InternalRegs::default()
            })),
            lcdint_gpio: None,
            lcdint_state: false,
            lcdint_deadline: Deadline::oneshot(Duration::ZERO),
        }
    }

    pub fn attach(&mut self, panel: LcdPanel, pin: Option<gpio::Sender>) {
        self.panel = Some(panel);
        self.lcdint_gpio = pin;
    }

    fn write_ram(&mut self, val: u16) {
        let mut ireg = self.ireg.write().unwrap();

        // Perform memory write, check we're within bounds
        if ireg.x_addr < MAX_WIDTH && ireg.y_addr < MAX_HEIGHT {
            let x = ireg.x_addr as usize;
            let y = ireg.y_addr as usize;
            self.gram.write().unwrap()[y * MAX_WIDTH as usize + x] = val;
        }

        // Compute x and y address wraps
        let x_wrap = ireg.x_addr == ireg.x_end;
        let next_x = match (x_wrap, ireg.agm.x_decr()) {
            (true, _) => ireg.x_start,
            (false, true) => ireg.x_addr.wrapping_sub(1),
            (false, false) => ireg.x_addr.wrapping_add(1),
        };

        let y_wrap = ireg.y_addr == ireg.y_end;
        let next_y = match (y_wrap, ireg.agm.y_decr()) {
            (true, _) => ireg.y_start,
            (false, true) => ireg.y_addr.wrapping_sub(1),
            (false, false) => ireg.y_addr.wrapping_add(1),
        };

        // Update x and y addresses
        if ireg.agm.swap_xy() {
            ireg.x_addr = next_x;
            if x_wrap {
                ireg.y_addr = next_y;
            }
        } else {
            ireg.y_addr = next_y;
            if y_wrap {
                ireg.x_addr = next_x;
            }
        }
    }

    fn make_render_callback(&self) -> RenderCallback {
        if self.panel.is_none() {
            panic!("make_render_callback called on LR38825 with no LCD panel set");
        }

        let gram = Arc::clone(&self.gram);
        let panel = self.panel;
        Box::new(move |buf: &mut Vec<u32>| -> (usize, usize) {
            let gram = *gram.read().unwrap();
            let new_buf = gram.iter().map(|&px| {
                let p = px.rotate_left(8);
                0xff000000 | rgb565_to_rgb888(p)
            });

            buf.splice(.., new_buf);

            if panel.unwrap().reverse_hor {
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
        let mut ireg = self.ireg.write().unwrap();
        ireg.cmd = val >> 8;
        let val = val as u8;

        if let 0..=254 = ireg.cmd {
            trace!(target: "LCD", "write_command cmd:{:x} val:0x{:x}({})", ireg.cmd, val, val);
        }
        match ireg.cmd {
            // 0: Oscillator stop
            0x1 => {}

            // VMD
            0x3 => {}

            0x8 => {}

            0x10 => ireg.hif1.write_bits(val),
            0x11 => ireg.hif2.write_bits(val),

            // ASX
            0x12 => {
                ireg.y_start = val;
                ireg.y_addr = val;
            }

            // ASY
            0x13 => {
                ireg.x_start = val;
                ireg.x_addr = val;
            }

            // Horizontal RAM address
            0x14 => ireg.y_addr = val,

            // AEX
            0x15 => ireg.y_end = val,

            // AEY
            0x16 => ireg.x_end = val,

            // Vertical RAM address
            0x17 => ireg.x_addr = val,

            0x18 => ireg.agm.write_bits(val),
            0x7E => ireg.display_setting.write_bits(val),

            // "V sync parameter transfer flag"
            //  0: TG parameter refresh
            0x7F => {}

            0x80 => ireg.display_control.write_bits(val),

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
        panic!("Not implemented");
    }

    fn write_data(&mut self, val: u16) -> MemResult<()> {
        self.write_ram(val);
        Ok(())
    }

    fn read_data(&mut self) -> MemResult<u16> {
        panic!("Not implemented");
    }

    fn render_callback(&self) -> RenderCallback {
        self.make_render_callback()
    }

    fn tick(&mut self) {
        if self.lcdint_gpio.is_none() || !self.lcdint_deadline.expired() {
            return;
        }

        let scan = self.ireg.read().unwrap().display_control.scan();
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
