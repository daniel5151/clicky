use crate::devices::prelude::*;

use std::sync::{Arc, RwLock};

use crate::devices::display::{LcdController, LcdPanel};
use crate::gui::RenderCallback;

const MAX_WIDTH: usize = 176;
const MAX_HEIGHT: usize = 220;
const GRAM_LEN: usize = MAX_WIDTH * MAX_HEIGHT;

#[derive(Debug, Default, Copy, Clone)]
struct InternalRegs {
    cmd: u16,
    // Display Control (R07)
    d: u8,
    dte: bool,
    gon: bool,
    // Horizontal/Vertical RAM Address Position (R44/R45)
    hsa: usize,
    hea: usize,
    vsa: usize,
    vea: usize,
    vertical_input: bool, // R03h.4
    horizontal_increment: i8, // R03h.5
    vertical_increment: i8, // R03h.6

    cur_x: usize,
    cur_y: usize,
}

/// Renesas Hd66789 176x240 color LCD Controller.
pub struct Hd66789 {
    panel: LcdPanel,
    gram: Arc<RwLock<[u16; GRAM_LEN]>>,
    ireg: Arc<RwLock<InternalRegs>>,
    rotated: bool,
}

impl std::fmt::Debug for Hd66789 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hd66789")
            .field("gram", &"[...]")
            .field("ireg", &self.ireg)
            .finish()
    }
}

impl Hd66789 {
    pub fn new(panel: LcdPanel) -> Hd66789 {
        Hd66789 {
            panel,
            gram: Arc::new(RwLock::new([0; GRAM_LEN])),
            ireg: Arc::new(RwLock::new(InternalRegs {
                hea: MAX_WIDTH - 1,
                vea: MAX_HEIGHT - 1,
                horizontal_increment: 1,
                vertical_increment: 1,
                ..InternalRegs::default()
            })),
            rotated: false,
        }
    }

    fn width(ireg: &InternalRegs) -> usize {
        ireg.hsa.abs_diff(ireg.hea) + 1
    }

    fn height(ireg: &InternalRegs) -> usize {
        ireg.vsa.abs_diff(ireg.vea) + 1
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

        let x = if ireg.hsa <= ireg.hea {
            ireg.hsa + ireg.cur_x
        } else {
            ireg.hsa - ireg.cur_x
        };

        let y = if ireg.vsa <= ireg.vea {
            ireg.vsa + ireg.cur_y
        } else {
            ireg.vsa - ireg.cur_y
        };

        Some((x, y))
    }

    fn advance(ireg: &mut InternalRegs) {
        let width = Self::width(ireg);
        let height = Self::height(ireg);
        match ireg.vertical_input {
            false => { // left to right
                if ireg.horizontal_increment == 1 {
                    ireg.cur_x += 1;
                    if ireg.cur_x >= width {
                        ireg.cur_x = 0;
                        ireg.cur_y += ireg.vertical_increment as usize;
                    }
                } else { // right to left
                    if ireg.cur_x == 0 {
                        ireg.cur_x = width - 1;
                        ireg.cur_y += ireg.vertical_increment as usize;
                    } else {
                        ireg.cur_x -= 1;
                    }
                }
            }
            true => { // top to bottom
                // if ireg.vertical_increment == 1 {
                //     ireg.cur_y += 1;
                //     if ireg.cur_y >= height {
                //         ireg.cur_y = 0;
                //         ireg.cur_x += ireg.horizontal_increment as usize;
                //     }
                // } else { // bottom to top
                    if ireg.cur_y == 0 {
                        ireg.cur_y = height - 1;
                        ireg.cur_x += ireg.horizontal_increment as usize;
                    } else {
                        ireg.cur_y -= 1;
                    }
                // }
            }
        }
    }

    fn write_ram(&mut self, val: u16) {
        let mut ireg = self.ireg.write().unwrap();
        let Some((x, y)) = Self::cursor_position(&ireg) else {
            return;
        };
        if x < MAX_WIDTH && y < MAX_HEIGHT {
            let idx = y * MAX_WIDTH + x;
            self.gram.write().unwrap()[idx] = val;
        }
        Self::advance(&mut ireg);
    }

    /// Returns a callback to update the framebuffer.
    ///
    /// The callback accepts a minifb framebuffer, and returns the rendered
    /// dimensions.
    fn make_render_callback(&self) -> RenderCallback {
        let gram = Arc::clone(&self.gram);
        let rotated = self.rotated;
        Box::new(move |buf: &mut Vec<u32>| -> (usize, usize) {
            let gram = *gram.read().unwrap();
            if !rotated {
                let new_buf = gram.iter().map(|&px| {
                    let p = px.rotate_left(8);
                    0xff000000
                        | (((p & 0xf800) as u32) << 8)
                        | (((p & 0x07e0) as u32) << 5)
                        | (((p & 0x001f) as u32) << 3)
                });

                buf.splice(.., new_buf);
                (MAX_WIDTH, MAX_HEIGHT)
            } else {
                buf.resize(MAX_WIDTH * MAX_HEIGHT, 0);
                for old_y in 0..MAX_HEIGHT {
                    for old_x in 0..MAX_WIDTH {
                        let px = gram[old_y * MAX_WIDTH + old_x];
                        let p = px.rotate_left(8);
                        let color =
                            0xff000000
                            | (((p & 0xf800) as u32) << 8)
                            | (((p & 0x07e0) as u32) << 5)
                            | (((p & 0x001f) as u32) << 3);

                        // 90° clockwise
                        let new_x = MAX_HEIGHT - 1 - old_y;
                        let new_y = old_x;

                        buf[new_y * MAX_HEIGHT + new_x] = color;
                    }
                }
                (MAX_HEIGHT, MAX_WIDTH)
            }
        })
    }

    fn handle_data_write(&mut self, val: u16) {
        let mut ireg = self.ireg.write().unwrap();
        match ireg.cmd {
            0x21 => {}
            0x22 => {}
            _ => {debug!(target: "LCD", "write_data cmd:{:x} val:0x{:x}({})", ireg.cmd, val, val);}
        }
        match ireg.cmd {
            0x03 => {
                ireg.vertical_input = val.get_bit(3);
                ireg.horizontal_increment = if val.get_bit(4) { 1 } else { -1 };
                ireg.vertical_increment = if val.get_bit(5) { 1 } else { -1 };
                Self::reset_cursor(&mut ireg);
            }
            // Display Control
            0x07 => {
                ireg.d = val.get_bits(0..=1) as u8;
                ireg.dte = val.get_bit(4);
                ireg.gon = val.get_bit(5);
            }
            // GRAM Address Set
            0x21 => {
                ireg.cur_x = val.get_bits(0..=7) as usize - ireg.hsa;
                ireg.cur_y = val.get_bits(8..=15) as usize - ireg.vsa;
            }
            // Write Data to GRAM
            0x22 => {
                drop(ireg);
                self.write_ram(val);
            }
            // Horizontal RAM Address Position (start..end)
            0x44 => {
                ireg.hsa = val.get_bits(0..=7) as usize;
                ireg.hea = val.get_bits(8..=15) as usize;
            }
            // Vertical RAM Address Position (start..end)
            0x45 => {
                ireg.vsa = val.get_bits(0..=7) as usize;
                ireg.vea = val.get_bits(8..=15) as usize;
            }
            _ => {}
        }
    }
}

impl LcdController for Hd66789 {
    fn write_command(&mut self, val: u16) -> MemResult<()> {
        let mut ireg = self.ireg.write().unwrap();
        ireg.cmd = val;

        if ireg.cmd > 0x5b {
            return Err(ContractViolation {
                msg: format!("Hd66789: set invalid LCD Command: {:#04x?}", val),
                severity: Error,
                stub_val: None,
            });
        }

        Ok(())
    }

    fn read_command(&mut self) -> MemResult<u16> {
        panic!("Unimplemented");
    }

    fn write_data(&mut self, val: u16) -> MemResult<()> {
        self.handle_data_write(val);
        Ok(())
    }

    fn read_data(&mut self) -> MemResult<u16> {
        panic!("Unimplemented");
    }

    fn render_callback(&self) -> RenderCallback {
        self.make_render_callback()
    }
}