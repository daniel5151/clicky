#[macro_use]
extern crate log;

use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use web_sys::{CanvasRenderingContext2d, ImageData};

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Trace).unwrap();
}

#[wasm_bindgen]
pub fn draw(ctx: &CanvasRenderingContext2d, width: u32, height: u32) -> Result<(), JsValue> {
    info!("called draw");
    let mut data = make_data(width, height);
    let data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&mut data), width, height)?;
    ctx.put_image_data(&data, 0.0, 0.0)
}

fn make_data(width: u32, height: u32) -> Vec<u8> {
    let mut data = Vec::new();

    for y in 0..height {
        for x in 0..width {
            data.push(((x as f32 / width as f32) * 255.) as u8);
            data.push(((y as f32 / height as f32) * 255.) as u8);
            data.push(255 as u8);
            data.push(255);
        }
    }

    data
}
