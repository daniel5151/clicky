use std::collections::HashMap;
use std::str::FromStr;

use crate::devices::input::Controls;
use crate::gui::{ButtonCallback, ScrollCallback};
use crate::signal::{self, gpio};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum IpodKey {
    Up,
    Down,
    Left,
    Right,
    Action,
    Hold,
}

#[derive(Default)]
pub struct IpodBinds {
    pub keys: HashMap<IpodKey, ButtonCallback>,
    pub wheel: Option<ScrollCallback>,
}

#[derive(Debug)]
pub struct IpodControls {
    pub hold: gpio::Sender,
    pub controls: Controls<signal::Master>,
}

impl IpodControls {
    pub fn into_binds(self) -> IpodBinds {
        let IpodControls {
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
        } = self;

        let mut controls = IpodBinds::default();

        controls.keys.insert(
            IpodKey::Hold,
            Box::new(move |pressed| {
                if pressed {
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

        connect_controls_btn!(IpodKey::Up, up);
        connect_controls_btn!(IpodKey::Down, down);
        connect_controls_btn!(IpodKey::Left, left);
        connect_controls_btn!(IpodKey::Right, right);
        connect_controls_btn!(IpodKey::Action, action);

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
                *wheel_data = (*wheel_data as i32 + (-dy * 2.0) as i32).rem_euclid(96) as u8;
            })
        });

        controls
    }
}

impl FromStr for IpodKey {
    type Err = String;

    // NOTE: the Hold switch is a latching, active-low GPIO rather than a keypad
    // signal, so it isn't one of the keys that can be parsed here.
    fn from_str(s: &str) -> Result<IpodKey, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "up" => Ok(IpodKey::Up),
            "down" => Ok(IpodKey::Down),
            "left" => Ok(IpodKey::Left),
            "right" => Ok(IpodKey::Right),
            "action" => Ok(IpodKey::Action),
            _ => Err(format!(
                "no such key: {:?} (expected one of: up, down, left, right, action)",
                s
            )),
        }
    }
}

/// Returns a handle to the signal driven by `key`
pub fn key_signal(controls: &Controls<signal::Master>, key: IpodKey) -> Option<signal::Master> {
    let signal = match key {
        IpodKey::Up => &controls.up,
        IpodKey::Down => &controls.down,
        IpodKey::Left => &controls.left,
        IpodKey::Right => &controls.right,
        IpodKey::Action => &controls.action,
        IpodKey::Hold => return None,
    };

    Some(signal.clone())
}
