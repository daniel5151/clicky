use super::super::{Ipod4g, Ipod4gControls};

use minifb::Key;

use crate::devices::platform::pp::Controls;
use crate::gui::minifb::MinifbControls;
use crate::gui::TakeControls;

impl TakeControls<MinifbControls> for Ipod4g {
    fn take_controls(&mut self) -> Option<MinifbControls> {
        let Ipod4gControls {
            mut hold,
            controls:
                Controls {
                    mut action,
                    mut up,
                    mut down,
                    mut left,
                    mut right,
                    wheel: (mut wheel_active, wheel_data),
                },
        } = self.controls.take()?;

        let mut controls = MinifbControls::new();

        controls.keymap.insert(
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

        macro_rules! connect_controls_btn {
            ($key:expr, $signal:expr) => {
                controls.keymap.insert(
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

        connect_controls_btn!(Key::Up, up);
        connect_controls_btn!(Key::Down, down);
        connect_controls_btn!(Key::Left, left);
        connect_controls_btn!(Key::Right, right);
        connect_controls_btn!(Key::Enter, action);

        // TODO: make sensitivity adjustable based on user's scroll speed
        controls.on_scroll = Some({
            Box::new(move |(_dx, dy)| {
                // HACK: the signal is edge-triggered
                // TODO: i really aught to rework how input works...
                if wheel_active.is_asserting() {
                    wheel_active.clear();
                } else {
                    wheel_active.assert();
                }

                let mut wheel_data = wheel_data.lock().unwrap();
                // from rockbox button-clickwheel.c
                // #define WHEELCLICKS_PER_ROTATION     96 /* wheelclicks per full rotation */
                *wheel_data = wheel_data.wrapping_add((-dy * 2.) as i8 as u8) % 96;
            })
        });

        Some(controls)
    }
}
