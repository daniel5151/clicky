use tinyaudio::prelude::*;

use clicky_core::gui::AudioCallback;

use crate::DynResult;

const SAMPLE_RATE: usize = 48_000;
const BUFFER_SIZE: usize = SAMPLE_RATE / 100;

pub fn start(mut render: AudioCallback) -> DynResult<OutputDevice> {
    let params = OutputDeviceParameters {
        sample_rate: SAMPLE_RATE,
        channels_count: 1,
        channel_sample_count: BUFFER_SIZE,
    };

    let device = run_output_device(params, move |out: &mut [f32]| {
        render(out, SAMPLE_RATE as u32)
    })
    .map_err(|e| format!("couldn't open an output device: {}", e))?;

    Ok(device)
}
