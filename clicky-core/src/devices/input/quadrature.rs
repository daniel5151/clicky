use std::time::Duration;

use relativity::Timeout;

use crate::executor::*;
use crate::signal::gpio;

use bit_field::BitField;

/// Time between two consecutive edges.
const STEP_PERIOD: Duration = Duration::from_micros(1000);

/// Upper bound on queued transitions, so that a scroll fling doesn't keep
/// playing out long after the user has stopped.
const MAX_PENDING: i32 = 96;

#[derive(Debug)]
pub struct QuadratureEncoder {
    steps_tx: async_channel::Sender<i32>,
}

impl QuadratureEncoder {
    pub fn new(a: gpio::Sender, b: gpio::Sender, task_spawner: &Spawner) -> QuadratureEncoder {
        let (steps_tx, steps_rx) = async_channel::unbounded();

        task_spawner
            .spawn(encoder_task(a, b, steps_rx))
            .expect("failed to spawn quadrature encoder task");

        QuadratureEncoder { steps_tx }
    }

    pub fn rotate(&self, steps: i32) {
        if steps != 0 {
            let _ = self.steps_tx.try_send(steps);
        }
    }
}

async fn encoder_task(
    mut a: gpio::Sender,
    mut b: gpio::Sender,
    steps_rx: async_channel::Receiver<i32>,
) {
    let mut phase = 0u8;
    let mut pending = 0i32;

    loop {
        if pending == 0 {
            pending = match steps_rx.recv().await {
                Ok(steps) => steps,
                Err(async_channel::RecvError) => return,
            };
        }
        while let Ok(steps) = steps_rx.try_recv() {
            pending = pending.saturating_add(steps);
        }
        pending = pending.clamp(-MAX_PENDING, MAX_PENDING);

        let dir = pending.signum();
        if dir == 0 {
            continue;
        }
        pending -= dir;
        phase = (phase as i32 + dir).rem_euclid(4) as u8;

        let gray = phase ^ (phase >> 1);
        a.set_level(gray.get_bit(0));
        b.set_level(gray.get_bit(1));

        Timeout::new(STEP_PERIOD).await;
    }
}
