//! Display-related devices.

use crate::devices::prelude::*;

use crate::gui::RenderCallback;

pub mod hd66753;

/// LCD Controller IC trait (eg. HD66753)
pub trait LcdPanel: std::fmt::Debug + Send + Sync {
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
}
