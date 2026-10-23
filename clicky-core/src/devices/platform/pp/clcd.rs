use crate::devices::prelude::*;

use crate::devices::display::LcdController;
use crate::gui::RenderCallback;
use bitfields::bitfield;

#[bitfield(u32, order = msb)]
struct Control {
    #[bits(2)]
    _reserved_31: u8,

    /// Set by the start write (0x3400_0000)
    running: bool,

    #[bits(default = true)]
    enable: bool,

    _reserved_27: bool,

    /// Polled by iPod photo bootloader before clearing Config at the end of a
    /// transfer
    done: bool,

    _reserved_25: bool,

    #[bits(default = true, access = ro)]
    fifo_not_full: bool,

    #[bits(16)]
    _reserved_23: u16,

    /// Flushes the FIFO
    #[bits(access = wo)]
    flush: bool,

    #[bits(3)]
    _reserved_6: u8,

    /// FIFO free slots, 8 slots total
    #[bits(4, default = 8, access = ro)]
    fifo_free_slots: u8,
}

// Observed values:
//  - 0xcc010001
//  - 0xc0010167
//  - 0xc0013547
//  - 0xc0013cef
//  - 0xc001f937
//
// ppdeluxe observations on iPod mini 1G
//
// pp> r32 70008a24
// 70008a24: 00000000
// pp> w32 70008a24 ffffffff
// 70008a24 <- ffffffff
// pp> r32 70008a24
// 70008a24: cf77ffff

#[bitfield(u32, order = msb)]
struct Config {
    /// Data / Command#
    data_ncmd: bool,

    /// Unknown configuration bits (0x4001 in Rockbox)
    #[bits(15)]
    _reserved: u16,

    /// Transaction size in bytes, minus one
    transaction_size: u16,
}

/// PP5020 color LCD controller.
///
/// The panel is driven over an 8-bit interface, so each 16-bit transfer takes
/// two accesses. Writes latch the high byte and commit on the second write;
/// reads return the high byte first and latch the low byte for the next read.
#[derive(Debug)]
pub struct ColorLcdBridge {
    config: Config,
    control: Control,

    /// Bytes left in the current block transfer.
    remaining: u32,

    write_byte_latch: Option<u32>,
    controller: Option<Box<dyn LcdController>>,
}

impl ColorLcdBridge {
    pub fn new() -> ColorLcdBridge {
        ColorLcdBridge {
            config: Config::new(),
            control: Control::new(),
            remaining: 0,
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
            0x00 => "?",
            0x04 => "?",
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
            0x20 => Err(StubRead(Trace, self.control.into_bits())),
            0x24 => Err(StubRead(Trace, self.config.into_bits())),
            _ => return Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x00 => Err(StubWrite(Trace, ())),
            0x04 => Err(StubWrite(Trace, ())),
            0x20 => {
                // `done` is owned by the engine: keep its current value.
                // The emulated FIFO drains instantly, so a flush has nothing to do.
                let done_mask = 1 << Control::DONE_OFFSET;
                self.control
                    .write_bits((val & !done_mask) | (self.control.into_bits() & done_mask));
                self.control.set_flush(false);
                Err(StubWrite(Trace, ()))
            }
            0x24 => {
                self.config = Config::from_bits(val);
                if self.config.data_ncmd() {
                    self.remaining = self.config.transaction_size() as u32 + 1;
                    self.control.set_done(false);
                }
                Err(StubWrite(Trace, ()))
            }
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
                if !self.config.data_ncmd() {
                    return self.controller()?.write_command(val as u16 >> 8);
                }

                if self.remaining == 0 {
                    return Err(ContractViolation {
                        msg: "Data written with no block transfer pending".into(),
                        severity: Warn,
                        stub_val: None,
                    });
                }

                if self.remaining > 2 {
                    let _ = self.controller()?.write_data(val as u16);
                    let _ = self.controller()?.write_data((val >> 16) as u16);
                    self.remaining = self.remaining.saturating_sub(4);
                } else {
                    let _ = self
                        .controller()?
                        .write_data(val as u16 >> 8 | (val as u16) << 8);
                    self.remaining = 0;
                }

                if self.remaining == 0 {
                    self.control.set_done(true);
                }

                Ok(())
            }
            _ => Err(Unexpected),
        }
    }
}
