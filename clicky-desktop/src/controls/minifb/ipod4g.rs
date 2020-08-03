use clicky_core::sys::ipod4g::{Ipod4gBinds, Ipod4gKey};
use minifb::Key;

use crate::backends::minifb::MinifbControls;

fn ipod4g_key_to_minifb(key: Ipod4gKey) -> Key {
    match key {
        Ipod4gKey::Up => Key::Up,
        Ipod4gKey::Down => Key::Down,
        Ipod4gKey::Left => Key::Left,
        Ipod4gKey::Right => Key::Right,
        Ipod4gKey::Action => Key::Enter,
        Ipod4gKey::Hold => Key::H,
    }
}

impl From<Ipod4gBinds> for MinifbControls {
    fn from(binds: Ipod4gBinds) -> MinifbControls {
        let Ipod4gBinds { keys, wheel } = binds;

        MinifbControls {
            keymap: keys
                .into_iter()
                .map(|(k, v)| (ipod4g_key_to_minifb(k), v))
                .collect(),
            on_scroll: wheel,
        }
    }
}
