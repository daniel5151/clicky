//! GUI implementations.

pub type RenderCallback = Box<dyn FnMut(&mut Vec<u32>) -> (usize, usize) + Send>;

pub trait TakeKeymap<K> {
    fn take_keymap(&mut self) -> Option<K>;
}

#[cfg(feature = "minifb")]
pub mod minifb;
