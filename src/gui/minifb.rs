use std::thread;

use crossbeam_channel as chan;
use minifb::{Key, Window, WindowOptions};

use crate::gui::RenderCallback;

#[derive(Debug)]
pub struct IPodMinifb {
    kill_tx: chan::Sender<()>,
}

impl IPodMinifb {
    /// (width, height) crops the framebuffer to the specific iPod model's
    /// screen size.
    pub fn new((width, height): (usize, usize), mut update_fb: RenderCallback) -> IPodMinifb {
        let (kill_tx, kill_rx) = chan::bounded(1);

        let thread = move || {
            let mut buffer: Vec<u32> = vec![0; width * height];
            let mut emu_buffer = Vec::new();

            let mut window = Window::new(
                "iPod 4g",
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
            // window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

            while window.is_open() && kill_rx.is_empty() && !window.is_key_down(Key::Escape) {
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
            .name("Hd66753 Renderer".into())
            .spawn(thread)
            .unwrap();

        IPodMinifb { kill_tx }
    }
}

impl Drop for IPodMinifb {
    fn drop(&mut self) {
        let _ = self.kill_tx.send(());
    }
}
