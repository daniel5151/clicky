use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

const CGRAM_BYTES: usize = 5544; // 168 * 132 at 2bpp

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
    // FIXME: not sure if there are separate latches for the command and data registers...
    byte_latch: Option<u8>,

    /// Index Register
    ir: u16,
    /// Address counter
    ac: usize, // only 12 bits, indexes into cgram
    /// Graphics RAM
    cgram: [u16; CGRAM_BYTES / 2],

    ireg: InternalRegs,
}

impl std::fmt::Debug for Hd66753 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        f.debug_struct("Hd66753")
            .field("byte_latch", &self.byte_latch)
            .field("ir", &self.ir)
            .field("ac", &self.ac)
            .field("cgram", &"[...]")
            .field("ireg", &self.ireg)
            .finish()
    }
}

impl Hd66753 {
    pub fn new_hle() -> Hd66753 {
        let mut this = Hd66753 {
            ir: 0,
            ac: 0,
            cgram: [0; CGRAM_BYTES / 2],
            byte_latch: None,
            ireg: InternalRegs::default(),
        };

        this.ireg.i_d = true; // increment address

        this
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
            0x11 => self.ac = val as usize % (CGRAM_BYTES / 2) / 2,
            // Write Data to CGRAM
            0x12 => {
                // Reference the Graphics Operation Function section of the manual for a
                // breakdown of what's happening here.

                // apply rotation
                let val = val.rotate_left(self.ireg.rt as u32 * 2);

                // apply the logical op
                let old_val = self.cgram[self.ac];
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
                self.cgram[self.ac] = val;

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

                self.ac %= (CGRAM_BYTES / 2) / 2;
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
                        msg: format!("set invalid LCD index register: {}", val),
                        severity: log::Level::Error,
                        stub_val: None,
                    });
                }
                log::trace!("LCD Command {:#06x?}", val);
                self.ir = val;
            }
            0x10 => {
                log::trace!("LCD Data {:#06x?}", val);
                self.handle_data(val)?;
            }
            _ => return Err(Unexpected),
        }

        Ok(())
    }
}
