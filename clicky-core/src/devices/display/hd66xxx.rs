use crate::devices::prelude::*;

use std::sync::{Arc, RwLock};

use crate::devices::display::LcdPanel;
use crate::gui::RenderCallback;

const MAX_WIDTH: usize = 220;
const MAX_HEIGHT: usize = 176;
const GRAM_LEN: usize = MAX_WIDTH * MAX_HEIGHT;

#[derive(Debug, Default, Copy, Clone)]
struct InternalRegs {
    cmd: u16,
    hsa: usize,
    hea: usize,
    vsa: usize,
    vea: usize,
    cur_x: usize,
    cur_y: usize,
}

// The unknown LCD controller used on iPod Photo
pub struct Hd66xxx {
    gram: Arc<RwLock<[u16; GRAM_LEN]>>,
    ireg: Arc<RwLock<InternalRegs>>,
}

impl std::fmt::Debug for Hd66xxx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hd66xxx")
            .field("gram", &"[...]")
            .field("ireg", &self.ireg)
            .finish()
    }
}

impl Hd66xxx {
    pub fn new() -> Hd66xxx {
        Hd66xxx {
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
        Hd66xxx::advance(&mut ireg);
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

impl LcdPanel for Hd66xxx {
    fn write_command(&mut self, val: u16) -> MemResult<()> {
        let mut ireg = self.ireg.write().unwrap();
        ireg.cmd = val >> 8;
        let val = val as u8;

        match ireg.cmd {
            0..=254 => {trace!(target: "LCD", "write_command cmd:{:x} val:0x{:x}({})", ireg.cmd, val, val);}
            _ => {}
        }
        match ireg.cmd {
            0x12 => {
                ireg.vsa = val as usize;
                ireg.cur_x = 0;
                ireg.cur_y = 0;
            }
            0x13 => {
                ireg.hea = val as usize;
                ireg.cur_x = 0;
                ireg.cur_y = 0;
            }
            0x15 => {
                ireg.vea = val as usize;
                ireg.cur_x = 0;
                ireg.cur_y = 0;
            }
            0x16 => {
                ireg.hsa = val as usize;
                ireg.cur_x = 0;
                ireg.cur_y = 0;
            }
            0x01 | 0x02 | 0x10 | 0x18 | 0x7e | 0x7f | 0x80 | 0xce | 0xef => {
            }
            invalid_cmd => {
                return Err(Fatal(format!(
                    "attempted to execute invalid LCD command {:#x?}",
                    invalid_cmd
                )))
            }
        }
        Ok(())
    }

    fn write_data(&mut self, val: u16) -> MemResult<()> {
        self.write_ram(val);
        Ok(())
    }

    fn render_callback(&self) -> RenderCallback {
        self.make_render_callback()
    }
}
