//! GUI related types and traits

/// `RenderCallback` is called with a RGBA Framebuffer, and returns the
/// dimensions of the image.
pub type RenderCallback =
    Box<dyn FnMut(/* rgba_framebuffer: */ &mut Vec<u32>) -> (usize, usize) + Send>;
/// `ButtonCallback` should be called whenever a button is pressed and released
/// (passing `true` and `false` respectively)
pub type ButtonCallback = Box<dyn FnMut(/* pressed: */ bool) + Send>;
/// `ScrollCallback` should be called on scroll, passing the delta in both
/// directions.
pub type ScrollCallback = Box<dyn FnMut(/* (dx, dy): */ (f32, f32)) + Send>;

pub trait TakeControls<K> {
    fn take_controls(&mut self) -> Option<K>;
}
