use crate::devices::prelude::*;

use crate::devices::display::LcdPanel;
use crate::gui::RenderCallback;

/// PP5020 monochrome LCD controller.
///
/// The panel is driven over an 8-bit interface, so each 16-bit transfer takes
/// two accesses. Writes latch the high byte and commit on the second write;
/// reads return the high byte first and latch the low byte for the next read.
#[derive(Debug)]
pub struct MonoLcdBridge {
    // FIXME: not sure if there are separate latches for the command and data
    // registers...
    write_byte_latch: Option<u8>,
    read_byte_latch: Option<u8>,

    panel: Box<dyn LcdPanel>,
}

impl MonoLcdBridge {
    pub fn new(panel: Box<dyn LcdPanel>) -> MonoLcdBridge {
        MonoLcdBridge {
            write_byte_latch: None,
            read_byte_latch: None,
            panel,
        }
    }

    /// Returns a callback to update the framebuffer.
    pub fn render_callback(&self) -> RenderCallback {
        self.panel.render_callback()
    }
}

impl Device for MonoLcdBridge {
    fn kind(&self) -> &'static str {
        "Mono LCD Bridge"
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

impl Memory for MonoLcdBridge {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        if offset == 0x0 {
            // bypass the latch
            //
            // Bit 15 is BUSY (iPodLinux: `lcd_busy_mask = 0x8000`), which
            // guests poll before each transfer. HACK: the emulated bridge
            // completes transfers instantly, so it is never busy.
            return Ok(0);
        }

        if let Some(val) = self.read_byte_latch.take() {
            return Ok(val as u32);
        }

        let val: u16 = match offset {
            0x8 => self.panel.read_command()?,
            0x10 => self.panel.read_data()?,
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

        // the iPod uses the controller via an 8-bit interface
        let val = val as u8; // FIXME: this should use trunc_to_u8, but it crashes...
        let val = match self.write_byte_latch.take() {
            None => {
                self.write_byte_latch = Some(val);
                return Ok(());
            }
            Some(hi) => (hi as u16) << 8 | (val as u16),
        };

        match offset {
            0x8 => self.panel.write_command(val),
            0x10 => self.panel.write_data(val),
            _ => Err(Unexpected),
        }
    }
}
