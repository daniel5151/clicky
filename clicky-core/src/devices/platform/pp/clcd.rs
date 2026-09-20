use crate::devices::prelude::*;

use crate::devices::display::LcdPanel;
use crate::gui::RenderCallback;

/// PP5020 color LCD controller.
///
/// The panel is driven over an 8-bit interface, so each 16-bit transfer takes
/// two accesses. Writes latch the high byte and commit on the second write;
/// reads return the high byte first and latch the low byte for the next read.
#[derive(Debug)]
pub struct ColorLcdBridge {
    write_byte_latch: Option<u32>,
    panel: Box<dyn LcdPanel>,
}

impl ColorLcdBridge {
    pub fn new(panel: Box<dyn LcdPanel>) -> ColorLcdBridge {
        ColorLcdBridge {
            write_byte_latch: None,
            panel,
        }
    }

    /// Returns a callback to update the framebuffer.
    pub fn render_callback(&self) -> RenderCallback {
        self.panel.render_callback()
    }
}

impl Device for ColorLcdBridge {
    fn kind(&self) -> &'static str {
        "Color LCD Bridge"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x0c => "Port",
            0x20 => "Control",
            0x24 => "Config",
            0x100 => "Data",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for ColorLcdBridge {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0c => Ok(0x0000_0000),
            0x20 => Ok(0x0500_0000),
            _ => return Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 | 0x04 | 0x20 | 0x24 => Ok(()),
            0x0c => {
                match self.write_byte_latch.take() {
                    None => {
                        self.write_byte_latch = Some(val);
                        return Ok(());
                    }
                    Some(hi) => {
                        let cmd16 = ((hi & 0xff) << 8 | (val & 0xff)) as u16;
                        if (val & 0xff00_0000) == 0x8000_0000 {
                            return self.panel.write_command(cmd16);
                        } else {
                            return self.panel.write_data(cmd16);
                        }
                    }
                };
            }
            0x100 => {
                let _ = self.panel.write_data(val as u16);
                let _ = self.panel.write_data((val >> 16) as u16);
                return Ok(());
            },
            _ => Err(Unexpected),
        }
    }
}
