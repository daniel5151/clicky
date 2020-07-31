use crate::devices::prelude::*;

use std::sync::{Arc, RwLock};

use crate::gui::RenderCallback;

const CGRAM_WIDTH: usize = 168;
const CGRAM_HEIGHT: usize = 132;
#[allow(dead_code)]
const CGRAM_BYTES: usize = (CGRAM_WIDTH * CGRAM_HEIGHT) * 2 / 8; // 168 * 132 at 2bpp

// Although the Hd66753 contains 5544 bytes of RAM (i.e: just the right amount
// to render 168 x 132 dots @ 2bpp), the RAM is _not_ linearly addressed!
// [The hard to read] Table 4 on page 24 of the manual describes the layout:
//
// `xx` denotes invalid RAM addresses.
//
//   AC   | 0x01 | 0x02 | ... | 0x14 | 0x15 | .... | 0x1f
// -------|------|------|-----|------|------|------|------
// 0x0000 |      |      |     |      |  xx  |  xx  |  xx
// 0x0020 |      |      |     |      |  xx  |  xx  |  xx
// 0x0040 |      |      |     |      |  xx  |  xx  |  xx
//  ...   |      |      |     |      |  xx  |  xx  |  xx
// 0x1060 |      |      |     |      |  xx  |  xx  |  xx
//
// This unorthodox mapping results in a couple annoyances:
// - the Address Counter auto-update feature has some funky wrapping logic
// - the Address Counter can't be used as a direct index into a
//   linearly-allocated CGRAM array of length (CGRAM_BYTES * 2)
//
// Point 2 can be mitigated by artificially allocating emulated CGRAM which
// includes the invalid RAM addresses. Yeah, it wastes some space, but
// it makes the code somewhat clearer, so whatever ¯\_(ツ)_/¯
const EMU_CGRAM_WIDTH: usize = 256;
const EMU_CGRAM_BYTES: usize = (EMU_CGRAM_WIDTH * CGRAM_HEIGHT) * 2 / 8;
const EMU_CGRAM_LEN: usize = EMU_CGRAM_BYTES / 2; // addressed as 16-bit words

// TODO: migrate to bit_field crate + mod reg { const X: usize = Y; ... }
#[derive(Debug, Default, Copy, Clone)]
struct InternalRegs {
    // Driver Output Control (R01)
    cms: bool,
    sgs: bool,
    nl: u8, // 5 bits
    // LCD-Driving-Waveform Control (R02)
    nw: u8, // 5 bits
    eor: bool,
    b_c: bool,
    // Power Control (R03)
    stb: bool,
    slp: bool,
    ap: u8, // 2 bits
    dc: u8, // 2 bits
    ps: u8, // 2 bits
    bt: u8, // 2 bits
    bs: u8, // 3 bits
    // Contrast Control (R04)
    vr: u8, // 3 bits
    ct: u8, // 7 bits
    // Entry / Rotation mode (R05/R06)
    i_d: bool,
    am: u8, // 2 bits
    lg: u8, // 2 bits
    rt: u8, // 3 bits
    // Display Control (R07)
    spt: bool,
    gsh: u8, // 2 bits
    gsl: u8, // 2 bits
    rev: bool,
    d: bool,
    // Cursor Control (R08)
    c: bool,
    cm: u8, // 2 bits
    // RAM Write Data Mask (R10)
    wm: u16,
    // Horizontal/Vertical Cursor Position (R0B/R0C)
    hs: u8,
    he: u8,
    vs: u8,
    ve: u8,
    // 1st/2nd Screen Driving Position (R0D/R0E)
    ss1: u8,
    se1: u8,
    ss2: u8,
    se2: u8,
}

/// Hitachi HD66753 168x132 monochrome LCD Controller.
pub struct Hd66753 {
    // FIXME: not sure if there are separate latches for the command and data registers...
    write_byte_latch: Option<u8>,
    read_byte_latch: Option<u8>,

    /// Index Register
    ir: u16,
    /// Address counter
    ac: usize, // only 12 bits, indexes into cgram
    /// Graphics RAM
    cgram: Arc<RwLock<[u16; EMU_CGRAM_LEN]>>,

    ireg: Arc<RwLock<InternalRegs>>,
}

impl std::fmt::Debug for Hd66753 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hd66753")
            .field("write_byte_latch", &self.write_byte_latch)
            .field("read_byte_latch", &self.read_byte_latch)
            .field("ir", &self.ir)
            .field("ac", &self.ac)
            .field("cgram", &"[...]")
            .field("ireg", &self.ireg)
            .finish()
    }
}

impl Hd66753 {
    pub fn new() -> Hd66753 {
        let cgram = Arc::new(RwLock::new([0; EMU_CGRAM_LEN]));
        let ireg = Arc::new(RwLock::new(InternalRegs {
            nl: 0b11111, // 168 x 132
            ..InternalRegs::default()
        }));

        Hd66753 {
            ir: 0,
            ac: 0,
            cgram,
            write_byte_latch: None,
            read_byte_latch: None,
            ireg,
        }
    }

    /// Returns a callback to update the framebuffer.
    ///
    /// The callback accepts a minifb framebuffer, and returns the rendered
    /// dimensions.
    pub fn render_callback(&self) -> RenderCallback {
        let cgram = Arc::clone(&self.cgram);
        let ireg = Arc::clone(&self.ireg);

        Box::new(move |buf: &mut Vec<u32>| -> (usize, usize) {
            // TODO: make palette configurable?
            #[allow(clippy::unreadable_literal)]
            const PALETTE: [u32; 4] = [0x000000, 0x686868, 0xb8b8b9, 0xffffff];

            // instead of holding the locks, just copy the data locally
            let cgram = *cgram.read().unwrap();
            let ireg = *ireg.read().unwrap();

            let height = match ireg.nl {
                0b11111 => 132,
                nl => (nl as usize + 1) * 8,
            };

            let cgram_window = cgram
                .chunks_exact(EMU_CGRAM_WIDTH * 2 / 8 / 2)
                .take(height)
                .flat_map(|row| row.iter().take(CGRAM_WIDTH * 2 / 8 / 2).rev());

            // TODO: implement cursor control

            let new_buf = cgram_window.flat_map(|w| {
                // every 16 bits = 8 pixels
                (0..8).rev().map(move |i| {
                    let idx = ((w >> (i * 2)) & 0b11) as usize;
                    if ireg.rev {
                        PALETTE[idx]
                    } else {
                        PALETTE[3 - idx]
                    }
                })
            });

            // replace in-place
            buf.splice(.., new_buf);

            assert_eq!(buf.len(), CGRAM_WIDTH * height);

            (CGRAM_WIDTH, height)
        })
    }

    fn handle_data_write(&mut self, val: u16) -> MemResult<()> {
        let mut ireg = self.ireg.write().unwrap();

        match self.ir {
            // Start Oscillation
            0x00 => {
                // TODO: track if oscillating?
            }
            // Driver output control
            0x01 => {
                ireg.cms = val.get_bit(9);
                ireg.sgs = val.get_bit(8);
                ireg.nl = val.get_bits(0..=4) as u8;
            }
            // LCD-Driving-Waveform Control
            0x02 => {
                ireg.nw = val.get_bits(0..=4) as u8;
                ireg.eor = val.get_bit(5);
                ireg.b_c = val.get_bit(6);
            }
            // Power Control
            0x03 => {
                ireg.stb = val.get_bit(0);
                ireg.slp = val.get_bit(1);
                ireg.ap = val.get_bits(2..=3) as u8;
                ireg.dc = val.get_bits(4..=5) as u8;
                ireg.ps = val.get_bits(6..=7) as u8;
                ireg.bt = val.get_bits(8..=9) as u8;
                ireg.bs = val.get_bits(10..=12) as u8;
            }
            // Contrast Control
            0x04 => {
                ireg.vr = val.get_bits(8..=10) as u8;
                ireg.ct = val.get_bits(0..=6) as u8;
                // TODO?: use Contrast Control bits to control rendered contrast
            }
            // Entry Mode
            0x05 => {
                ireg.i_d = val.get_bit(4);
                ireg.am = val.get_bits(2..=3) as u8;
                ireg.lg = val.get_bits(0..=1) as u8;

                if ireg.am == 0b11 {
                    return Err(ContractViolation {
                        msg: "0b11 is an invalid LCD EntryMode:AM value".into(),
                        severity: Error,
                        stub_val: None,
                    });
                }
            }
            // Rotation
            0x06 => ireg.rt = val.get_bits(0..=2) as u8,
            // Display Control
            0x07 => {
                ireg.spt = val.get_bit(8);
                ireg.gsh = val.get_bits(4..=5) as u8;
                ireg.gsl = val.get_bits(2..=3) as u8;
                ireg.rev = val.get_bit(1);
                ireg.d = val.get_bit(0);
            }
            // Cursor Control
            0x08 => {
                ireg.cm = val.get_bits(0..=1) as u8;
                ireg.c = val.get_bit(2);

                if ireg.c {
                    return Err(ContractViolation {
                        msg: "cursor mode is enabled, but not implemented!".into(),
                        severity: Warn,
                        stub_val: None,
                    });
                } else {
                    return Err(ContractViolation {
                        msg: "cursor mode is now disabled".into(),
                        severity: Info,
                        stub_val: None,
                    });
                }
            }
            // NOOP
            0x09 => {}
            // NOOP
            0x0a => {}
            // Horizontal Cursor Position
            0x0b => {
                ireg.hs = val.get_bits(0..=7) as u8;
                ireg.he = val.get_bits(8..=15) as u8;
            }
            // Vertical Cursor Position
            0x0c => {
                ireg.vs = val.get_bits(0..=7) as u8;
                ireg.ve = val.get_bits(8..=15) as u8;
            }
            // 1st Screen Driving Position
            0x0d => {
                ireg.ss1 = val.get_bits(0..=7) as u8;
                ireg.se1 = val.get_bits(8..=15) as u8;
            }
            // 1st Screen Driving Position
            0x0e => {
                ireg.ss2 = val.get_bits(0..=7) as u8;
                ireg.se2 = val.get_bits(8..=15) as u8;
            }
            // NOTE: 0x0f isn't listed as a valid command.
            // 0x0f => {},
            // RAM Write Data Mask
            0x10 => ireg.wm = val,
            // RAM Address Set
            0x11 => self.ac = val as usize % 0x1080,
            // Write Data to CGRAM
            0x12 => {
                // Reference the Graphics Operation Function section of the manual for a
                // breakdown of what's happening here.

                let mut cgram = self.cgram.write().unwrap();

                // apply rotation
                let val = val.rotate_left(ireg.rt as u32 * 2);

                // apply the logical op
                let old_val = cgram[self.ac];
                let val = match ireg.lg {
                    0b00 => val, // replace
                    0b01 => old_val | val,
                    0b10 => old_val & val,
                    0b11 => old_val ^ val,
                    _ => unreachable!(),
                };

                // apply the write mask
                let val = (old_val & ireg.wm) | (val & !ireg.wm);

                // do the write
                cgram[self.ac] = val;

                // increment the ac appropriately
                let dx_ac = match ireg.am {
                    0b00 => 1,
                    0b01 => return Err(Fatal("unimplemented: vertical CGRAM write".into())),
                    0b10 => {
                        return Err(Fatal("unimplemented: two-word vertical CGRAM write".into()))
                    }
                    0b11 => return Err(Fatal("EntryMode:AM cannot be set to 0b11".into())),
                    _ => unreachable!(),
                };

                self.ac = match ireg.i_d {
                    true => self.ac.wrapping_add(dx_ac),
                    false => self.ac.wrapping_sub(dx_ac),
                };

                self.ac %= 0x1080;

                // ... and handle wrapping behavior
                if self.ac & 0x1f > 0x14 {
                    self.ac = match ireg.i_d {
                        true => (self.ac & !0x1f) + 0x20,
                        false => (self.ac & !0x1f) + 0x14,
                    };
                }

                self.ac %= 0x1080;
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

    fn handle_data_read(&mut self) -> MemResult<u16> {
        match self.ir {
            // device code read
            0x0 => Ok(0b0000011101010011), // hardcoded from spec sheet
            invalid_cmd => Err(Fatal(format!(
                "attempted to execute invalid LCD command {:#x?}",
                invalid_cmd
            ))),
        }
    }
}

impl Device for Hd66753 {
    fn kind(&self) -> &'static str {
        "HD 66753"
    }

    fn probe(&self, offset: u32) -> Probe {
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
        if offset == 0x0 {
            // bypass the latch
            return Ok(0); // HACK: Emulated LCD is never busy
        }

        if let Some(val) = self.read_byte_latch.take() {
            return Ok(val as u32);
        }

        let val: u16 = match offset {
            // XXX: not currently tracking driving raster-row position
            0x8 => self.ireg.read().unwrap().ct as u16,
            0x10 => self.handle_data_read()?,
            _ => return Err(Unexpected),
        };

        self.read_byte_latch = Some(val as u8); // latch lower 8 bits
        Ok((val >> 8) as u32) // returning the higher 8 bits first
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        if offset == 0x0 {
            // bypass the latch
            return Err(StubWrite(Error, ()));
        }

        let val = val as u8; // the iPod uses the controller via an 8-bit interface
        let val = match self.write_byte_latch.take() {
            None => {
                self.write_byte_latch = Some(val as u8);
                return Ok(());
            }
            Some(hi) => (hi as u16) << 8 | (val as u16),
        };

        match offset {
            0x8 => {
                self.ir = val;

                if self.ir > 0x12 {
                    return Err(ContractViolation {
                        msg: format!("set invalid LCD Command: {:#04x?}", val),
                        severity: Error,
                        stub_val: None,
                    });
                }

                Ok(())
            }
            0x10 => Ok(self.handle_data_write(val)?),
            _ => Err(Unexpected),
        }
    }
}
