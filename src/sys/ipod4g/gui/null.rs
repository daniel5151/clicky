use super::super::Ipod4g;
use crate::gui::TakeKeymap;

impl TakeKeymap<()> for Ipod4g {
    fn take_keymap(&mut self) -> Option<()> {
        Some(())
    }
}
