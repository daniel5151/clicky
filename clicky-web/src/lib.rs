#[macro_use]
extern crate log;

use std::io::{self, Read};

use flate2::read::GzDecoder;
use wasm_bindgen::prelude::*;

use clicky_core::block::{self, BlockDev};
use clicky_core::gui::{RenderCallback, TakeControls};
use clicky_core::sys::ipod4g::{BootKind, Ipod4g, Ipod4gBinds, Ipod4gKey};

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("[{}] {}", record.target(), message))
        })
        .level(log::LevelFilter::Debug)
        .level_for("clicky_core", log::LevelFilter::Trace)
        .level_for("MMIO", log::LevelFilter::Info)
        // .level_for("IRQ", log::LevelFilter::Trace)
        .chain(fern::Output::call(console_log::log))
        .apply()
        .unwrap();
}

fn gzip_decompress(data: &[u8]) -> io::Result<Box<[u8]>> {
    let mut decoder = GzDecoder::new(data);
    let mut data = Vec::new();
    decoder.read_to_end(&mut data)?;
    Ok(data.into_boxed_slice())
}

#[wasm_bindgen]
pub struct Ipod4gContainer {
    system: Ipod4g,
    render_callback: RenderCallback,
    framebuffer: Vec<u32>,
}

#[wasm_bindgen]
impl Ipod4gContainer {
    #[wasm_bindgen(constructor)]
    pub fn new(fw: &[u8], disk: &[u8]) -> Result<Ipod4gContainer, JsValue> {
        let fw =
            gzip_decompress(fw).map_err(|e| format!("could not decompress firmware: {}", e))?;
        let disk =
            gzip_decompress(disk).map_err(|e| format!("could not decompress disk image: {}", e))?;

        debug!("decompressed fw and disk image");

        let hdd: Box<dyn BlockDev> = Box::new(block::backend::Mem::new(disk));

        let system = Ipod4g::new(
            hdd,
            None,
            BootKind::HLEBoot {
                fw_file: io::Cursor::new(fw),
            },
        )
        .map_err(|e| e.to_string())?;
        debug!("built system");

        let render_callback = system.render_callback();
        Ok(Ipod4gContainer {
            system,
            render_callback,
            framebuffer: Vec::new(),
        })
    }

    #[wasm_bindgen]
    pub fn take_controls(&mut self) -> Result<Ipod4gController, JsValue> {
        Ok(Ipod4gController {
            controls: self
                .system
                .take_controls()
                .ok_or_else(|| "can only take controls once")?,
        })
    }

    #[wasm_bindgen]
    pub fn get_frame(&mut self) -> Frame {
        let (width, height) = (self.render_callback)(&mut self.framebuffer);

        Frame {
            width,
            height,
            data: unsafe {
                let mut buf = self.framebuffer.clone();
                let len = buf.len();
                let ptr = buf.as_mut_ptr();
                std::mem::forget(buf); // avoid double-free
                Vec::from_raw_parts(ptr as *mut u8, len * 4, len * 4).into_boxed_slice()
            },
        }
    }

    #[wasm_bindgen]
    pub fn run(&mut self, cycles: usize) -> Result<(), JsValue> {
        self.system
            .run_cycles(cycles)
            .map_err(|e| format!("fatal error: {:?}", e).into())
    }
}

#[wasm_bindgen]
pub struct Frame {
    #[wasm_bindgen]
    pub width: usize,
    #[wasm_bindgen]
    pub height: usize,
    // gotta use a getter
    data: Box<[u8]>,
}

#[wasm_bindgen]
impl Frame {
    #[wasm_bindgen]
    pub fn get_data(self) -> Box<[u8]> {
        self.data
    }
}

#[wasm_bindgen]
pub enum Ipod4gKeyKind {
    Up,
    Down,
    Left,
    Right,
    Action,
    Hold,
}

impl From<Ipod4gKeyKind> for Ipod4gKey {
    fn from(wasm_key: Ipod4gKeyKind) -> Ipod4gKey {
        match wasm_key {
            Ipod4gKeyKind::Up => Ipod4gKey::Up,
            Ipod4gKeyKind::Down => Ipod4gKey::Down,
            Ipod4gKeyKind::Left => Ipod4gKey::Left,
            Ipod4gKeyKind::Right => Ipod4gKey::Right,
            Ipod4gKeyKind::Action => Ipod4gKey::Action,
            Ipod4gKeyKind::Hold => Ipod4gKey::Hold,
        }
    }
}

#[wasm_bindgen]
pub struct Ipod4gController {
    controls: Ipod4gBinds,
}

#[wasm_bindgen]
impl Ipod4gController {
    #[wasm_bindgen]
    pub fn on_keydown(&mut self, key: Ipod4gKeyKind) {
        if let Some(cb) = self.controls.keys.get_mut(&key.into()) {
            cb(true)
        }
    }

    #[wasm_bindgen]
    pub fn on_keyup(&mut self, key: Ipod4gKeyKind) {
        if let Some(cb) = self.controls.keys.get_mut(&key.into()) {
            cb(false)
        }
    }

    #[wasm_bindgen]
    pub fn on_scroll(&mut self, dx: f32, dy: f32) {
        if let Some(ref mut cb) = self.controls.wheel {
            cb((dx, dy))
        }
    }
}
