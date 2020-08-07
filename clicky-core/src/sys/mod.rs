//! Concrete system implementations.

pub mod ipod4g;

#[allow(dead_code)]
mod size_asserts {
    use super::*;

    /// Default Rust wasm stack size
    const DEFAULT_WASM_STACK_SIZE: usize = 0x100000;

    /// Arbitrary size limit on systems, just to make sure they don't blow out
    /// the stack when being constructed.
    const MAX_SYS_SIZE: usize = DEFAULT_WASM_STACK_SIZE / 4;

    const_assert!(std::mem::size_of::<ipod4g::Ipod4g>() < MAX_SYS_SIZE);
}
