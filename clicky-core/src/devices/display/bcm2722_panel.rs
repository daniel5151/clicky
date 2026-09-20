use crate::devices::prelude::*;
use crate::gui::RenderCallback;
use crate::devices::display::LcdPanel;

use std::sync::{Arc, RwLock};

const MAX_WIDTH: usize = 320;
const MAX_HEIGHT: usize = 240;
const GRAM_LEN: usize = MAX_WIDTH * MAX_HEIGHT;

#[derive(Debug, Default, Copy, Clone)]
struct InternalRegs {
    cmd: u32,
    hsa: usize,
    hea: usize,
    vsa: usize,
    vea: usize,
    cur_x: usize,
    cur_y: usize,
}

#[derive(Debug)]
pub struct Bcm2722Panel {
    gram: Arc<RwLock<[u16; GRAM_LEN]>>,
    ireg: Arc<RwLock<InternalRegs>>,
}

impl Bcm2722Panel {
    pub fn new() -> Bcm2722Panel {
        Bcm2722Panel {
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
        let pos = ireg.cur_y * width + ireg.cur_x + 2;

        ireg.cur_x = pos % width;
        ireg.cur_y = pos / width;

        if ireg.cur_y >= height {
            ireg.cur_y = height;
        }
    }

    fn write_ram(&mut self, val: u32) {
        let mut ireg = self.ireg.write().unwrap();
        let idx = {
            let x = ireg.hsa + ireg.cur_x;
            let y = ireg.vsa + ireg.cur_y;
            y * MAX_WIDTH + x
        };
        if idx + 1 < GRAM_LEN {
            let mut gram = self.gram.write().unwrap();
            gram[idx    ] =  val        as u16;
            gram[idx + 1] = (val >> 16) as u16;
        }
        Bcm2722Panel::advance(&mut ireg);
    }

    pub fn make_render_callback(&self) -> RenderCallback {
        let gram = Arc::clone(&self.gram);
        Box::new(move |buf: &mut Vec<u32>| -> (usize, usize) {
            let gram = *gram.read().unwrap();
            let new_buf = gram.iter().map(|&px| {
                let p = px;
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

impl LcdPanel for Bcm2722Panel {
    fn write_command32(&mut self, val: u32) -> MemResult<()> {
        let mut ireg = self.ireg.write().unwrap();
        Ok(ireg.cmd = val)
    }

    fn write_data32(&mut self, val: u32) -> MemResult<()> {
        let mut ireg = self.ireg.write().unwrap();
        let index = (ireg.cmd >> 1) - 0x7_0000;

        match index {
            2..=14 => {trace!(target: "LCD", "write_data cmd:{:x} val:0x{:x}({})", ireg.cmd, val, val);}
            _ => {}
        }
        match index {
            2 => ireg.hsa = val as usize,
            4 => ireg.vsa = val as usize,
            6 => ireg.hea = val as usize,
            8 => ireg.vea = val as usize,
            0 | 22 ..= 0x25810 => { // 16 .. (16 + 320x240x2)
                drop(ireg);
                self.write_ram(val);
            }
            _ => {}
        }
        Ok(())
    }

    fn render_callback(&self) -> RenderCallback {
        self.make_render_callback()
    }
}
