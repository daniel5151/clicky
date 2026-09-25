use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz, Type};
use relativity::Instant;

use crate::gui::AudioCallback;

use super::super::pwm::PwmSink;

const PIEZO_RESONANCE_HZ: f32 = 4000.0;
const PIEZO_Q: f32 = 9.0;

/// Output gain (linear)
const OUTPUT_GAIN: f32 = 0.030_618;

/// Default biquad coefficients
const UNTUNED: Coefficients<f32> = Coefficients {
    a1: 0.0,
    a2: 0.0,
    b0: 0.0,
    b1: 0.0,
    b2: 0.0,
};

/// Threshold to disable audio rendering.
const SILENCE: f32 = 1e-15;

/// How far behind the emulator the audio thread renders
const LATENCY_US: u64 = 20_000;

/// How far the audio clock may fall behind the emulator
const MAX_LAG_US: u64 = 100_000;

/// Channel updates in flight between the two threads.
const EVENT_QUEUE: usize = 64;

#[derive(Debug, Clone, Copy, Default)]
struct Event {
    at_us: u64,
    enabled: bool,
    frequency: f32,
    duty: f32,
}

fn sample_at(at_us: u64, clock_us: u64, sample_rate: u32) -> usize {
    ((at_us.saturating_sub(clock_us) * u64::from(sample_rate)) / 1_000_000) as usize
}

#[derive(Debug)]
pub struct Piezo {
    events: Arc<Mutex<VecDeque<Event>>>,
    epoch: Instant,
}

impl Piezo {
    pub fn new() -> (Piezo, PiezoAudio) {
        let events = Arc::new(Mutex::new(VecDeque::with_capacity(EVENT_QUEUE)));
        let epoch = Instant::now();

        let piezo = Piezo {
            events: Arc::clone(&events),
            epoch,
        };

        let audio = PiezoAudio {
            events,
            epoch,
            clock_us: None,
            state: Event::default(),
            phase: 0.0,
            disc: DirectForm2Transposed::new(UNTUNED),
        };

        (piezo, audio)
    }
}

impl PwmSink for Piezo {
    fn update(&mut self, enabled: bool, frequency: f32, duty: f32) {
        let at_us = self.epoch.elapsed().as_micros() as u64;

        trace!(
            target: "PWM",
            "piezo: enabled={} {:.0} Hz duty {:.2} at {} us",
            enabled, frequency, duty, at_us
        );

        // Dropped if the audio thread has fallen far enough behind to fill the
        // queue, in which case it is already rendering stale state anyway.
        if let Ok(mut events) = self.events.lock() {
            if events.len() < EVENT_QUEUE {
                events.push_back(Event {
                    at_us,
                    enabled,
                    frequency,
                    duty,
                });
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PiezoAudio {
    events: Arc<Mutex<VecDeque<Event>>>,
    epoch: Instant,
    /// Host time of the next sample to render.
    clock_us: Option<u64>,
    /// What the channel is doing as of `clock_us`.
    state: Event,
    phase: f32,
    disc: DirectForm2Transposed<f32>,
}

impl PiezoAudio {
    pub fn audio_callback(&self) -> AudioCallback {
        let mut synth = self.clone();
        Box::new(move |out: &mut [f32], sample_rate: u32| synth.render(out, sample_rate))
    }

    fn render(&mut self, out: &mut [f32], sample_rate: u32) {
        let rate = sample_rate as f32;

        if let Ok(coeffs) =
            Coefficients::from_params(Type::BandPass, rate.hz(), PIEZO_RESONANCE_HZ.hz(), PIEZO_Q)
        {
            self.disc.update_coefficients(coeffs);
        }

        let now_us = self.epoch.elapsed().as_micros() as u64;
        let target = now_us.saturating_sub(LATENCY_US);
        let clock_us = match self.clock_us {
            None => target,
            Some(clock) if target.saturating_sub(clock) > MAX_LAG_US => target,
            Some(clock) => clock,
        };

        // Held for the whole buffer: the producer only ever takes the lock long
        // enough to push, and dropping it per sample would cost more than it
        // saves. Events that land past the end of the buffer stay queued.
        let mut queue = self.events.try_lock().ok();

        for (i, sample) in out.iter_mut().enumerate() {
            if let Some(queue) = queue.as_mut() {
                while queue
                    .front()
                    .map_or(false, |e| sample_at(e.at_us, clock_us, sample_rate) <= i)
                {
                    self.state = queue.pop_front().unwrap();
                }
            }

            let state = self.state;

            self.phase = (self.phase + state.frequency / rate).fract();
            let pulse = if self.phase < state.duty { 1.0 } else { -1.0 };
            let drive = if state.enabled { pulse } else { 0.0 };

            *sample = (self.disc.run(drive) * OUTPUT_GAIN).clamp(-1.0, 1.0);
        }

        if !self.state.enabled && self.disc.s1.abs() < SILENCE && self.disc.s2.abs() < SILENCE {
            self.disc.reset_state();
        }

        self.clock_us = Some(clock_us + (out.len() as u64 * 1_000_000) / u64::from(sample_rate));
    }
}
