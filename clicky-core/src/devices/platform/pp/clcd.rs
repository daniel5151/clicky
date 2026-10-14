use crate::devices::prelude::*;

use crate::devices::display::LcdController;
use crate::gui::RenderCallback;

/// PP5020 color LCD controller.
///
/// The panel is driven over an 8-bit interface, so each 16-bit transfer takes
/// two accesses. Writes latch the high byte and commit on the second write;
/// reads return the high byte first and latch the low byte for the next read.
#[derive(Debug)]
pub struct ColorLcdBridge {
    config: u32,
    write_byte_latch: Option<u32>,
    controller: Option<Box<dyn LcdController>>,
}

impl ColorLcdBridge {
    pub fn new() -> ColorLcdBridge {
        ColorLcdBridge {
            config: 0,
            write_byte_latch: None,
            controller: None,
        }
    }

    pub fn attach(&mut self, controller: Box<dyn LcdController>) {
        self.controller = Some(controller);
    }

    fn controller(&mut self) -> MemResult<&mut Box<dyn LcdController>> {
        self.controller
            .as_mut()
            .ok_or_else(|| Fatal("no LCD controller attached to the color LCD bridge".into()))
    }

    /// Returns a callback to update the framebuffer.
    pub fn render_callback(&self) -> Option<RenderCallback> {
        match &self.controller {
            Some(controller) => Some(controller.render_callback()),
            None => None,
        }
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
            0x00 => Err(StubRead(Trace, 0)),
            0x04 => Err(StubRead(Trace, 0)),
            0x0c => Err(StubRead(Trace, 0)),
            0x20 => Err(StubRead(Trace, 0x0500_0000)),
            0x24 => Err(StubRead(Trace, self.config)),
            _ => return Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => Err(StubWrite(Trace, ())),
            0x04 => Err(StubWrite(Trace, ())),
            0x20 => Err(StubWrite(Trace, ())),
            0x24 => Err(StubWrite(Trace, self.config = val)),
            0x0c => {
                match self.write_byte_latch.take() {
                    None => {
                        self.write_byte_latch = Some(val);
                        return Ok(());
                    }
                    Some(hi) => {
                        let cmd16 = ((hi & 0xff) << 8 | (val & 0xff)) as u16;
                        if (val & 0xff00_0000) == 0x8000_0000 {
                            return self.controller()?.write_command(cmd16);
                        } else {
                            return self.controller()?.write_data(cmd16);
                        }
                    }
                };
            }
            0x100 => {
                if self.config.get_bit(16) {
                    if self.config.get_bit(1) {
                        // a bit chosen randomly between 0xcc010001 (w8) vs 0xc0010167/0xc0013547/0xc0013cef/0xc001f937 (2x w16)
                        let _ = self.controller()?.write_data(val as u16);
                        let _ = self.controller()?.write_data((val >> 16) as u16);
                    } else {
                        let _ = self.controller()?.write_data(val as u16 >> 8 | (val as u16) << 8);
                    }
                    return Ok(());
                } else {
                    return self.controller()?.write_command(val as u16 >> 8);
                }
            },
            _ => Err(Unexpected),
        }
    }
}