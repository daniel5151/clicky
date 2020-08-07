use super::{Ipod4g, Ipod4gControls};

use std::collections::HashMap;

use crate::devices::platform::pp::Controls;
use crate::gui::{ButtonCallback, ScrollCallback, TakeControls};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum Ipod4gKey {
    Up,
    Down,
    Left,
    Right,
    Action,
    Hold,
}

#[derive(Default)]
pub struct Ipod4gBinds {
    pub keys: HashMap<Ipod4gKey, ButtonCallback>,
    pub wheel: Option<ScrollCallback>,
}

impl TakeControls<Ipod4gBinds> for Ipod4g {
    fn take_controls(&mut self) -> Option<Ipod4gBinds> {
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

        let mut controls = Ipod4gBinds::default();

        controls.keys.insert(
            Ipod4gKey::Hold,
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
                controls.keys.insert(
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

        connect_controls_btn!(Ipod4gKey::Up, up);
        connect_controls_btn!(Ipod4gKey::Down, down);
        connect_controls_btn!(Ipod4gKey::Left, left);
        connect_controls_btn!(Ipod4gKey::Right, right);
        connect_controls_btn!(Ipod4gKey::Action, action);

        // TODO: make sensitivity adjustable based on user's scroll speed
        controls.wheel = Some({
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
