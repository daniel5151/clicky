//! Display-related devices.

use crate::devices::prelude::*;

use crate::gui::RenderCallback;

pub mod hd66753;
pub mod hd66xxx;
pub mod hd66789;
pub mod bcm2722;
pub mod bcm2722_panel;

/// LCD Controller IC trait (eg. HD66753)
pub trait LcdPanel: std::fmt::Debug + Send + Sync {
    /// Select a register / issue a command (i.e: write the Index Register).
    fn write_command(&mut self, _val: u16) -> MemResult<()> { Ok(()) }
    fn write_command32(&mut self, _val: u32) -> MemResult<()> { Ok(()) }

    /// Read back the command register.
    fn read_command(&mut self) -> MemResult<u16> { Ok(0) }
    fn read_command32(&mut self) -> MemResult<u32> { Ok(0) }

    /// Write to the currently selected register.
    fn write_data(&mut self, _val: u16) -> MemResult<()> { Ok(()) }
    fn write_data32(&mut self, _val: u32) -> MemResult<()> { Ok(()) }

    /// Read from the currently selected register.
    fn read_data(&mut self) -> MemResult<u16> { Ok(0) }
    fn read_data32(&mut self) -> MemResult<u32> { Ok(0) }

    /// Returns a callback which renders the panel's framebuffer.
    ///
    /// The callback accepts a framebuffer, and returns the rendered dimensions.
    fn render_callback(&self) -> RenderCallback;
}
