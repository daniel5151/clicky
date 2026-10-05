use clicky_core::gui::{IpodBinds, IpodKey};
use minifb::Key;

use crate::backends::minifb::MinifbControls;

fn ipod_key_to_minifb(key: IpodKey) -> Key {
    match key {
        IpodKey::Up => Key::Up,
        IpodKey::Down => Key::Down,
        IpodKey::Left => Key::Left,
        IpodKey::Right => Key::Right,
        IpodKey::Action => Key::Enter,
        IpodKey::Hold => Key::H,
    }
}

impl From<IpodBinds> for MinifbControls {
    fn from(binds: IpodBinds) -> MinifbControls {
        let IpodBinds { keys, wheel } = binds;

        MinifbControls {
            keymap: keys
                .into_iter()
                .map(|(k, v)| (ipod_key_to_minifb(k), v))
                .collect(),
            on_scroll: wheel,
        }
    }
}
