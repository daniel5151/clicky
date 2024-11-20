use std::collections::HashMap;
use std::sync::mpsc as chan;

use minifb::{Key, Window, WindowOptions};

use clicky_core::gui::{ButtonCallback, RenderCallback, ScrollCallback};

pub struct MinifbControls {
    pub keymap: HashMap<Key, ButtonCallback>,
    pub on_scroll: Option<ScrollCallback>,
}

#[derive(Debug)]
pub struct MinifbRenderer {}

impl MinifbRenderer {
    /// (width, height) crops the framebuffer to the specified screen size
    /// (starting from the top-left corner)
    pub fn run(
        title: &'static str,
        (width, height): (usize, usize),
        mut update_fb: RenderCallback,
        controls: impl Into<MinifbControls>,
        kill_rx: chan::Receiver<()>,
    ) {
        let mut controls = controls.into();

        let mut buffer: Vec<u32> = vec![0; width * height];
        let mut emu_buffer = Vec::new();

        let mut window = Window::new(
            title,
            width,
            height,
            WindowOptions {
                scale: minifb::Scale::X4,
                resize: true,
                ..WindowOptions::default()
            },
        )
        .expect("could not create minifb window");

        window.set_target_fps(60);

        'ui_loop: while window.is_open() && kill_rx.try_recv().is_err() {
            let keys = window.get_keys_pressed(minifb::KeyRepeat::Yes);
            for k in keys {
                if k == Key::Escape {
                    break 'ui_loop;
                }

                if let Some(cb) = controls.keymap.get_mut(&k) {
                    cb(true)
                }
            }

            let keys = window.get_keys_released();
            for k in keys {
                if let Some(cb) = controls.keymap.get_mut(&k) {
                    cb(false)
                }
            }

            if let Some(scroll) = window.get_scroll_wheel() {
                if let Some(ref mut on_scroll) = controls.on_scroll {
                    on_scroll(scroll)
                }
            }

            // update the framebuffer
            let (w, _h) = update_fb(&mut emu_buffer);

            // crop the emulated buffer
            let new_buf = emu_buffer
                .chunks_exact(w)
                .take(height)
                .flat_map(|row| row.iter().take(width))
                .copied();
            buffer.splice(.., new_buf);

            window
                .update_with_buffer(&buffer, width, height)
                .expect("could not update minifb window");
        }
    }
}
