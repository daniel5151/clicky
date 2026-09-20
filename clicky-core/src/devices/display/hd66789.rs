use crate::devices::prelude::*;

use std::sync::{Arc, RwLock};

use crate::devices::display::LcdPanel;
use crate::gui::RenderCallback;

const MAX_WIDTH: usize = 176;
const MAX_HEIGHT: usize = 240;
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
    // Current possition
    cur_x: usize,
    cur_y: usize,
}

/// Renesas Hd66789 176x240 color LCD Controller.
pub struct Hd66789 {
    gram: Arc<RwLock<[u16; GRAM_LEN]>>,
    ireg: Arc<RwLock<InternalRegs>>,
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
    pub fn new() -> Hd66789 {
        Hd66789 {
            gram: Arc::new(RwLock::new([0; GRAM_LEN])),
            ireg: Arc::new(RwLock::new(InternalRegs {
                hea: (MAX_WIDTH - 1),
                vea: (MAX_HEIGHT - 1),
                ..InternalRegs::default()
            })),
        }
    }

    fn advance(ireg: &mut InternalRegs) {
        let width = ireg.hea - ireg.hsa + 1;
        let height = ireg.vea - ireg.vsa + 1;
        let pos = ireg.cur_y * width + ireg.cur_x + 1;

        ireg.cur_x = pos % width;
        ireg.cur_y = pos / width;

        if ireg.cur_y >= height {
            ireg.cur_y = height;
        }
    }

    fn write_ram(&mut self, val: u16) {
        let mut ireg = self.ireg.write().unwrap();
        let idx = {
            let x = ireg.hsa + ireg.cur_x;
            let y = ireg.vsa + ireg.cur_y;
            y * MAX_WIDTH + x
        };
        if idx < GRAM_LEN {
            self.gram.write().unwrap()[idx] = val;
        }
        Hd66789::advance(&mut ireg);
    }

    /// Returns a callback to update the framebuffer.
    ///
    /// The callback accepts a minifb framebuffer, and returns the rendered
    /// dimensions.
    fn make_render_callback(&self) -> RenderCallback {
        let gram = Arc::clone(&self.gram);
        let ireg = Arc::clone(&self.ireg);
        Box::new(move |buf: &mut Vec<u32>| -> (usize, usize) {
            let gram = *gram.read().unwrap();
            let ireg = *ireg.read().unwrap();
            let new_buf = gram.iter().map(|&px| {
                let p = px.rotate_left(8);
                if !ireg.d.get_bit(0) || !ireg.gon || !ireg.dte {
                    0xff000000
                } else {
                    0xff000000
                        | (((p & 0xf800) as u32) << 8)
                        | (((p & 0x07e0) as u32) << 5)
                        | (((p & 0x001f) as u32) << 3)
                }
            });

            buf.splice(.., new_buf);
            (MAX_WIDTH, MAX_HEIGHT)
        })
    }

    fn handle_data_write(&mut self, val: u16) {
        let mut ireg = self.ireg.write().unwrap();
        match ireg.cmd {
            0x22 => {}
            _ => {trace!(target: "LCD", "write_data cmd:{:x} val:0x{:x}({})", ireg.cmd, val, val);}
        }
        match ireg.cmd {
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

impl LcdPanel for Hd66789 {
    fn write_command(&mut self, val: u16) -> MemResult<()> {
        let mut ireg = self.ireg.write().unwrap();
        ireg.cmd = val;

        if ireg.cmd > 0x46 {
            return Err(ContractViolation {
                msg: format!("set invalid LCD Command: {:#04x?}", val),
                severity: Error,
                stub_val: None,
            });
        }

        Ok(())
    }

    fn write_data(&mut self, val: u16) -> MemResult<()> {
        self.handle_data_write(val);
        Ok(())
    }

    fn render_callback(&self) -> RenderCallback {
        self.make_render_callback()
    }
}
