//! Display-related devices.

use crate::devices::prelude::*;

use crate::gui::RenderCallback;

pub mod hd66753;
pub mod hd66789;
pub mod lr38825;

/// LCD Controller IC trait (eg. HD66753)
pub trait LcdController: std::fmt::Debug + Send + Sync {
    /// Select a register / issue a command (i.e: write the Index Register).
    fn write_command(&mut self, val: u16) -> MemResult<()>;

    /// Read back the command register.
    fn read_command(&mut self) -> MemResult<u16>;

    /// Write to the currently selected register.
    fn write_data(&mut self, val: u16) -> MemResult<()>;

    /// Read from the currently selected register.
    fn read_data(&mut self) -> MemResult<u16>;

    /// Returns a callback which renders the panel's framebuffer.
    ///
    /// The callback accepts a framebuffer, and returns the rendered dimensions.
    fn render_callback(&self) -> RenderCallback;

    /// Called on each tick
    fn tick(&mut self) {}
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LcdPanel {
    /// Panel pixel width
    pub width: usize,

    /// Panel pixel height
    pub height: usize,

    /// Reversed horizontal axis
    ///
    /// Panel segments are connected in the reverse order
    pub reverse_hor: bool,
}

pub fn rgb565_to_rgb888(p: u16) -> u32 {
    // RGB565 -> RGB888 conversion courtesy of
    // https://stackoverflow.com/questions/2442576/how-does-one-convert-16-bit-rgb565-to-24-bit-rgb888
    let r5 = p.get_bits(11..=15) as u32;
    let g6 = p.get_bits(5..=10) as u32;
    let b5 = p.get_bits(0..=4) as u32;

    let r8 = (r5 * 255) / 31;
    let g8 = (g6 * 255) / 63;
    let b8 = (b5 * 255) / 31;

    (r8 << 16) | (g8 << 8) | b8
}
