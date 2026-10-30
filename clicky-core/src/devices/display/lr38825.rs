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

use crate::devices::display::{LcdController, LcdPanel};
use crate::gui::RenderCallback;

const MAX_WIDTH: usize = 220;
const MAX_HEIGHT: usize = 176;
const GRAM_LEN: usize = MAX_WIDTH * MAX_HEIGHT;

#[derive(Debug, Default, Copy, Clone)]
struct InternalRegs {
    cmd: u16,
    x_start: usize,
    x_end: usize,
    y_start: usize,
    y_end: usize,
    flip_x: bool,

    cur_x: usize,
    cur_y: usize,

    mirror_x: bool,
    mirror_y: bool,
    swap_xy: bool,
}

pub struct LR38825 {
    panel: Option<LcdPanel>,
    gram: Arc<RwLock<[u16; GRAM_LEN]>>,
    ireg: Arc<RwLock<InternalRegs>>,
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
        }
    }

    pub fn attach(&mut self, panel: LcdPanel) {
        self.panel = Some(panel);
    }

    fn width(ireg: &InternalRegs) -> usize {
        ireg.x_start.abs_diff(ireg.x_end) + 1
    }

    fn height(ireg: &InternalRegs) -> usize {
        ireg.y_start.abs_diff(ireg.y_end) + 1
    }

    /// Map a vertical window address (ASY / AEY) to an iPod X coordinate.
    fn map_x(ireg: &InternalRegs, val: u8) -> usize {
        match ireg.flip_x {
            true => (MAX_WIDTH - 1).saturating_sub(val as usize),
            false => val as usize,
        }
    }

    fn reset_cursor(ireg: &mut InternalRegs) {
        ireg.cur_x = 0;
        ireg.cur_y = 0;
    }

    fn cursor_position(ireg: &InternalRegs) -> Option<(usize, usize)> {
        let width = Self::width(ireg);
        let height = Self::height(ireg);

        if ireg.cur_x >= width || ireg.cur_y >= height {
            return None;
        }

        let x = if ireg.x_start <= ireg.x_end {
            ireg.x_start + ireg.cur_x
        } else {
            ireg.x_start - ireg.cur_x
        };

        let y = if ireg.y_start <= ireg.y_end {
            ireg.y_start + ireg.cur_y
        } else {
            ireg.y_start - ireg.cur_y
        };

        Some((x, y))
    }

    fn write_ram(&mut self, val: u16) {
        let mut ireg = self.ireg.write().unwrap();
        let width = Self::width(&ireg);
        let height = Self::height(&ireg);

        let Some((x, y)) = Self::cursor_position(&ireg) else {
            return;
        };

        if x < MAX_WIDTH && y < MAX_HEIGHT {
            let idx = y * MAX_WIDTH + x;
            self.gram.write().unwrap()[idx] = val;
        }

        if ireg.swap_xy {
            ireg.cur_x += 1;
            if ireg.cur_x >= width {
                ireg.cur_x = 0;
                ireg.cur_y += 1;
            }
        } else {
            ireg.cur_y += 1;
            if ireg.cur_y >= height {
                ireg.cur_y = 0;
                ireg.cur_x += 1;
            }
        }
    }

    fn make_render_callback(&self) -> RenderCallback {
        let gram = Arc::clone(&self.gram);
        Box::new(move |buf: &mut Vec<u32>| -> (usize, usize) {
            let gram = *gram.read().unwrap();
            let new_buf = gram.iter().map(|&px| {
                let p = px.rotate_left(8);
                0xff000000
                    | (((p & 0xf800) as u32) << 8)
                    | (((p & 0x07e0) as u32) << 5)
                    | (((p & 0x001f) as u32) << 3)
            });

            buf.splice(.., new_buf);
            (MAX_WIDTH, MAX_HEIGHT)
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

            // HIF1 Layout:
            //  0-1: ?
            //  2: Display RAM memory address incr/decr
            //  3: PDF0 (?)
            //  4-5: ?
            //  6: 0=RGB, 1=BGR
            0x10 => {
                ireg.flip_x = val.get_bit(2);
            }

            // HIF2
            0x11 => {}

            // ASX
            0x12 => {
                ireg.y_start = val as usize;
                Self::reset_cursor(&mut ireg);
            }

            // ASY
            0x13 => {
                ireg.x_start = Self::map_x(&ireg, val);
                Self::reset_cursor(&mut ireg);
            }

            // Horizontal RAM address
            0x14 => {}

            // AEX
            0x15 => {
                ireg.y_end = val as usize;
                Self::reset_cursor(&mut ireg);
            }

            // AEY
            0x16 => {
                ireg.x_end = Self::map_x(&ireg, val);
                Self::reset_cursor(&mut ireg);
            }

            // Vertical RAM address
            0x17 => {}

            // AGM layout:
            //  0: Mirror X
            //  1: Mirror Y
            //  2: Swap X/Y
            0x18 => {
                ireg.mirror_x = val.get_bit(0);
                ireg.mirror_y = val.get_bit(1);
                ireg.swap_xy = val.get_bit(2);
            }

            // "Display displaying setting register" (sic)
            0x7E => {}

            // "V sync parameter transfer flag"
            //  0: TG parameter refresh
            0x7F => {}

            // "Display displaying control register" (sic)
            //  0: Display on
            //  1-7: ?
            0x80 => {}

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
}
