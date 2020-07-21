use super::super::{Ipod4g, Ipod4gControls};

use minifb::Key;

use crate::devices::platform::pp::KeypadSignals;
use crate::gui::minifb::MinifbKeymap;
use crate::gui::TakeKeymap;

impl TakeKeymap<MinifbKeymap> for Ipod4g {
    fn take_keymap(&mut self) -> Option<MinifbKeymap> {
        let Ipod4gControls {
            mut hold,
            keypad:
                KeypadSignals {
                    mut action,
                    mut up,
                    mut down,
                    mut left,
                    mut right,
                },
        } = self.controls.take()?;

        let mut controls = MinifbKeymap::new();
        controls.insert(
            Key::H, // H for Hold
            Box::new(move |pressed| {
                if pressed {
                    // toggle on and off
                    match hold.is_set_high() {
                        false => hold.set_high(),
                        true => hold.set_low(),
                    }
                }
            }),
        );

        macro_rules! connect_keypad_btn {
            ($key:expr, $signal:expr) => {
                controls.insert(
                    $key,
                    Box::new(move |pressed| {
                        if pressed {
                            $signal.assert()
                        } else {
                            $signal.clear()
                        }
                    }),
                );
            };
        }

        connect_keypad_btn!(Key::Up, up);
        connect_keypad_btn!(Key::Down, down);
        connect_keypad_btn!(Key::Left, left);
        connect_keypad_btn!(Key::Right, right);
        connect_keypad_btn!(Key::Enter, action);

        Some(controls)
    }
}
