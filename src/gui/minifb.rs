use std::collections::HashMap;
use std::sync::mpsc as chan;
use std::thread;

use minifb::{Key, Window, WindowOptions};

use crate::gui::RenderCallback;

pub type MinifbKeymap = HashMap<Key, Box<dyn FnMut(bool) + Send>>;

#[derive(Debug)]
pub struct MinifbRenderer {
    kill_tx: chan::Sender<()>,
}

impl MinifbRenderer {
    /// (width, height) crops the framebuffer to the specified screen size
    /// (starting from the top-left corner)
    pub fn new(
        title: &'static str,
        (width, height): (usize, usize),
        mut update_fb: RenderCallback,
        mut controls: MinifbKeymap,
    ) -> MinifbRenderer {
        let (kill_tx, kill_rx) = chan::channel();

        let thread = move || {
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

            // ~60 fps
            window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

            'ui_loop: while window.is_open() && kill_rx.try_recv().is_err() {
                if let Some(keys) = window.get_keys_pressed(minifb::KeyRepeat::Yes) {
                    for k in keys {
                        if k == Key::Escape {
                            break 'ui_loop;
                        }

                        if let Some(cb) = controls.get_mut(&k) {
                            cb(true)
                        }
                    }
                }

                if let Some(keys) = window.get_keys_released() {
                    for k in keys {
                        if let Some(cb) = controls.get_mut(&k) {
                            cb(false)
                        }
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

            // XXX: don't just std::process::exit when LCD window closes.
            std::process::exit(0)
        };

        let _handle = thread::Builder::new()
            .name("minifb ui".into())
            .spawn(thread)
            .unwrap();

        MinifbRenderer { kill_tx }
    }
}

impl Drop for MinifbRenderer {
    fn drop(&mut self) {
        let _ = self.kill_tx.send(());
    }
}
