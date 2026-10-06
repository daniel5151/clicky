//! Concrete system implementations.

use crate::error::FatalMemResult;
use crate::gui::{IpodKey, RenderCallback, TakeControls};

pub mod ipod4g;
pub mod ipodmini1g;

pub trait System: TakeControls + std::fmt::Debug + Send {
    fn model_name(&self) -> &'static str;

    fn screen_size(&self) -> (usize, usize);

    fn render_callback(&self) -> RenderCallback;

    /// Set keys hold at boot
    fn set_hold_keys(&mut self, keys: Vec<IpodKey>);

    /// Run the system, returning successfully on "graceful exit"
    /// (e.g: power-off).
    fn run(&mut self) -> FatalMemResult<()>;

    /// Run the system, returning successfully on "graceful exit" (e.g:
    /// power-off). This method will return after the specified number of cycles
    /// have been executed.
    fn run_cycles(&mut self, cycles: usize) -> FatalMemResult<()>;

    /// Freeze the system such that it stops executing code. Called prior to
    /// spawning a "post-mortem" GDB session.
    ///
    /// WARNING - THERE IS NO WAY TO "THAW" A FROZEN SYSTEM!
    fn freeze(&mut self);
}

pub enum BootKind<F: std::io::Read + std::io::Seek> {
    ColdBoot,
    HLEBoot { fw_file: F },
}

#[allow(dead_code)]
mod size_asserts {
    use super::*;

    /// Default Rust wasm stack size
    const DEFAULT_WASM_STACK_SIZE: usize = 0x100000;

    /// Arbitrary size limit on systems, just to make sure they don't blow out
    /// the stack when being constructed.
    const MAX_SYS_SIZE: usize = DEFAULT_WASM_STACK_SIZE / 4;

    const_assert!(std::mem::size_of::<ipod4g::Ipod4g>() < MAX_SYS_SIZE);
    const_assert!(std::mem::size_of::<ipodmini1g::IpodMini1g>() < MAX_SYS_SIZE);
}
