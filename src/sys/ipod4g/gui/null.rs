use super::super::Ipod4g;
use crate::gui::TakeControls;

impl TakeControls<()> for Ipod4g {
    fn take_controls(&mut self) -> Option<()> {
        Some(())
    }
}
