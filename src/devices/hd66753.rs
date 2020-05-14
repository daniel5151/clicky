use std::sync::{Arc, RwLock};
use std::thread;

use crossbeam_channel as chan;
use minifb::{Key, Window, WindowOptions};

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

const CGRAM_WIDTH: usize = 168;
const CGRAM_HEIGHT: usize = 132;
const CGRAM_BYTES: usize = (CGRAM_WIDTH * CGRAM_HEIGHT) * 2 / 8; // 168 * 132 at 2bpp

// While the Hd66753 contains 5544 bytes of RAM (just enough to render 168 x 132
// as 2bpp), the RAM is _not_ linearly addressed! Table 4 on page 24 of the
// manual describes the layout:
//
// `xx` denotes invalid RAM addresses.
//
//        | 0x01 | 0x02 | .. | 0x14 | 0x15 | .... | 0x1f
// -------|------|------|----|------|------|------|------
// 0x0000 |      |      |    |      |  xx  |  xx  |  xx
// 0x0020 |      |      |    |      |  xx  |  xx  |  xx
// 0x0040 |      |      |    |      |  xx  |  xx  |  xx
//  ...   |      |      |    |      |  xx  |  xx  |  xx
// 0x1060 |      |      |    |      |  xx  |  xx  |  xx
//
// This unorthodox mapping results in a couple annoyances:
// - The Address Counter auto-update feature relies on some obtuse wrapping code
// - the Address Counter can't be used as a direct index into a
//   linearly-allocated emulated CGRAM array
//
// Point 2 can be mitigated by artificially allocating emulated CGRAM which
// includes the invalid RAM addresses. Yeah, it wastes some space, but
// it makes the code easier, so whatever ¯\_(ツ)_/¯
const EMU_CGRAM_WIDTH: usize = 256;
const EMU_CGRAM_BYTES: usize = (EMU_CGRAM_WIDTH * CGRAM_HEIGHT) * 2 / 8;
const EMU_CGRAM_LEN: usize = EMU_CGRAM_BYTES / 2; // addressed as 16-bit words

#[allow(clippy::unreadable_literal)]
const PALETTE: [u32; 4] = [0x000000, 0x686868, 0xb8b8b9, 0xffffff];

#[derive(Debug)]
struct Hd66753Renderer {
    kill_tx: chan::Sender<()>,
}

impl Hd66753Renderer {
    fn new(
        width: usize,
        height: usize,
        cgram: Arc<RwLock<[u16; EMU_CGRAM_LEN]>>,
    ) -> Hd66753Renderer {
        let _ = CGRAM_BYTES;

        let (kill_tx, kill_rx) = chan::bounded(1);

        let thread = move || {
            let mut buffer: Vec<u32> = vec![0; width * height];

            let mut window = Window::new(
                "iPod 4g",
                width,
                height,
                WindowOptions {
                    scale: minifb::Scale::X4,
                    resize: true,
                    ..WindowOptions::default()
                },
            )
            .expect("could not create minifb window");

            // ~60 fps
            window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

            while window.is_open() && kill_rx.is_empty() && !window.is_key_down(Key::Escape) {
                let cgram = cgram.read().unwrap().clone(); // avoid holding a lock

                // Only translate the chunk of CGRAM corresponding to visible pixels (given by
                // the connected display's width / height

                let cgram_window = cgram
                    .chunks_exact(EMU_CGRAM_WIDTH * 2 / 8 / 2)
                    .take(height)
                    .flat_map(|row| row.iter().take(width * 2 / 8 / 2));

                let new_buf = cgram_window.flat_map(|w| {
                    // every 16 bits = 8 pixels
                    (0..8).rev().map(move |i| {
                        let idx = ((w >> (i * 2)) & 0b11) as usize;
                        // TODO: invert-screen functionality
                        PALETTE[3 - idx]
                    })
                });

                // replace in-place
                buffer.splice(.., new_buf);

                assert_eq!(buffer.len(), width * height);

                window
                    .update_with_buffer(&buffer, width, height)
                    .expect("could not update minifb window");
            }

            // XXX: don't just std::process::exit when LCD window closes.
            std::process::exit(0)
        };

        let _handle = thread::Builder::new()
            .name("Hd66753 Renderer".into())
            .spawn(thread)
            .unwrap();

        Hd66753Renderer { kill_tx }
    }
}

impl Drop for Hd66753Renderer {
    fn drop(&mut self) {
        let _ = self.kill_tx.send(());
    }
}

#[derive(Default, Debug)]
struct InternalRegs {
    // Entry / Rotation mode (page 32)
    i_d: bool,
    am: u8, // 2 bits
    lg: u8, // 2 bits
    rt: u8, // 3 bits
    // RAM Write Data Mask (page 36)
    wm: u16,
}

/// Hitachi HD66753 168x132 monochrome LCD Controller
pub struct Hd66753 {
    renderer: Hd66753Renderer,

    // FIXME: not sure if there are separate latches for the command and data registers...
    byte_latch: Option<u8>,

    /// Index Register
    ir: u16,
    /// Address counter
    ac: usize, // only 12 bits, indexes into cgram
    /// Graphics RAM
    cgram: Arc<RwLock<[u16; EMU_CGRAM_LEN]>>,

    ireg: InternalRegs,
}

impl std::fmt::Debug for Hd66753 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        f.debug_struct("Hd66753")
            .field("renderer", &self.renderer)
            .field("byte_latch", &self.byte_latch)
            .field("ir", &self.ir)
            .field("ac", &self.ac)
            .field("cgram", &"[...]")
            .field("ireg", &self.ireg)
            .finish()
    }
}

impl Hd66753 {
    pub fn new_hle(width: usize, height: usize) -> Hd66753 {
        let cgram = Arc::new(RwLock::new([0; EMU_CGRAM_LEN]));

        Hd66753 {
            renderer: Hd66753Renderer::new(width, height, Arc::clone(&cgram)),
            ir: 0,
            ac: 0,
            cgram,
            byte_latch: None,
            ireg: InternalRegs {
                // FIXME: not sure if this is supposed to be set or not, but it works
                i_d: true,
                ..InternalRegs::default()
            },
        }
    }

    fn handle_data(&mut self, val: u16) -> MemResult<()> {
        match self.ir {
            // Entry Mode
            0x05 => {
                self.ireg.i_d = val & 0b10000 != 0;
                self.ireg.am = ((val & 0b1100) >> 2) as u8;
                self.ireg.lg = ((val & 0b0011) >> 0) as u8;

                if self.ireg.am == 0b11 {
                    return Err(ContractViolation {
                        msg: "0b11 is an invalid LCD AM value".into(),
                        severity: log::Level::Error,
                        stub_val: None,
                    });
                }
            }
            // Rotation
            0x06 => self.ireg.rt = (val & 0b111) as u8,
            // RAM Write Data Mask
            0x10 => self.ireg.wm = val,
            // RAM Address Set
            0x11 => self.ac = val as usize % 0x1080,
            // Write Data to CGRAM
            0x12 => {
                // Reference the Graphics Operation Function section of the manual for a
                // breakdown of what's happening here.

                let mut cgram = self.cgram.write().unwrap();

                // apply rotation
                let val = val.rotate_left(self.ireg.rt as u32 * 2);

                // apply the logical op
                let old_val = cgram[self.ac];
                let val = match self.ireg.lg {
                    0b00 => val, // replace
                    0b01 => old_val | val,
                    0b10 => old_val & val,
                    0b11 => old_val ^ val,
                    _ => unreachable!(),
                };

                // apply the write mask
                let val = (old_val & self.ireg.wm) | (val & !self.ireg.wm);

                // do the write
                cgram[self.ac] = val;

                // increment the ac appropriately
                let dx_ac = match self.ireg.am {
                    0b00 => 1,
                    0b01 => todo!("implement vertical CGRAM write"),
                    0b10 => todo!("implement two-word vertical CGRAM write"),
                    _ => unreachable!(),
                };

                self.ac = match self.ireg.i_d {
                    true => self.ac.wrapping_add(dx_ac),
                    false => self.ac.wrapping_sub(dx_ac),
                };

                self.ac %= 0x1080;

                // ... and handle wrapping behavior
                if self.ac & 0x1f > 0x14 {
                    self.ac = match self.ireg.i_d {
                        true => (self.ac & !0x1f) + 0x20,
                        false => (self.ac & !0x1f) + 0x14,
                    };
                }

                // log::debug!("new ac {:#06x?}", self.ac);

                self.ac %= 0x1080;
            }
            x if x < 0x12 => unimplemented!("LCD command {} isn't implemented", x),
            _ => unreachable!(),
        }

        Ok(())
    }
}

impl Device for Hd66753 {
    fn kind(&self) -> &'static str {
        "HD 66753"
    }

    fn probe(&self, offset: u32) -> Probe<'_> {
        let reg = match offset {
            0x0 => "LCD Control",
            0x8 => "LCD Command",
            0x10 => "LCD Data",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for Hd66753 {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        // FIXME: reads should probably be latched?
        match offset {
            0x0 => Ok(0), // HACK: Emulated LCD is never busy
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        let val = val as u8; // the iPod uses the controller using it's 8-bit interface

        let val = match self.byte_latch.take() {
            None => {
                self.byte_latch = Some(val as u8);
                return Ok(());
            }
            Some(hi) => (hi as u16) << 8 | (val as u16),
        };

        match offset {
            0x8 => {
                if val > 0x12 {
                    return Err(ContractViolation {
                        msg: format!("Trying to set invalid LCD Command: {}", val),
                        severity: log::Level::Error,
                        stub_val: None,
                    });
                }
                self.ir = val;
            }
            0x10 => self.handle_data(val)?,
            _ => return Err(Unexpected),
        }

        Ok(())
    }
}
