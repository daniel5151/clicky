//! GUI implementations.

pub type RenderCallback = Box<dyn FnMut(&mut Vec<u32>) -> (usize, usize) + Send>;

pub trait TakeControls<K> {
    fn take_controls(&mut self) -> Option<K>;
}

#[cfg(feature = "minifb")]
pub mod minifb;
