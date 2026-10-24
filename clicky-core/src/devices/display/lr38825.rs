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
    mirror: bool,
    vertical_input: usize,

    cur_x: usize,
    cur_y: usize,
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

    fn advance(ireg: &mut InternalRegs) {
        let width = Self::width(ireg);
        let height = Self::height(ireg);
        match ireg.vertical_input {
            6 => {
                // left to right / right to left
                ireg.cur_x += 1;
                if ireg.cur_x >= width {
                    ireg.cur_x = 0;
                    ireg.cur_y += 1;
                }
            }
            0 => {
                // top to bottom
                ireg.cur_y += 1;
                if ireg.cur_y >= height {
                    ireg.cur_y = 0;
                    ireg.cur_x += 1;
                }
            }
            mode => {
                error!(target: "LCD", "LR38825: unsupported memory access mode {:x}", mode);
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
            0x10 => {
                ireg.mirror = !val.get_bit(2);
            }
            0x12 => {
                ireg.y_start = val as usize;
                Self::reset_cursor(&mut ireg);
            }
            0x13 => {
                if ireg.mirror {
                    ireg.x_start = MAX_WIDTH - val as usize - 1;
                } else {
                    ireg.x_end = val as usize;
                }
                Self::reset_cursor(&mut ireg);
            }
            0x15 => {
                ireg.y_end = val as usize;
                Self::reset_cursor(&mut ireg);
            }
            0x16 => {
                if ireg.mirror {
                    ireg.x_end = MAX_WIDTH - val as usize - 1;
                } else {
                    ireg.x_start = val as usize;
                }
                Self::reset_cursor(&mut ireg);
            }
            0x18 => {
                ireg.vertical_input = val as usize;
            }
            0x01 | 0x02 | 0x7e | 0x7f | 0x80 | 0xce | 0xef => {}
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
