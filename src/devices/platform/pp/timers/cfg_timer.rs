use crate::devices::prelude::*;

use std::time::{Duration, Instant};

use futures::future::{self, Either};
use pin_utils::pin_mut;

use crate::signal::irq;

#[derive(Debug, Copy, Clone)]
enum InterrupterState {
    Oneshot { next: Instant },
    Repeating { next: Instant, period: Duration },
    Disabled,
}

async fn interrupter_task(mut irq: irq::Sender, msg_rx: async_channel::Receiver<InterrupterState>) {
    let mut state = InterrupterState::Disabled;

    loop {
        let interrupt = match state {
            InterrupterState::Disabled => Either::Left(future::pending()),
            InterrupterState::Oneshot { next } => {
                Either::Right(async_timer::new_timer(next - Instant::now()))
            }
            InterrupterState::Repeating { next, .. } => {
                Either::Right(async_timer::new_timer(next - Instant::now()))
            }
        };

        let msg_fut = msg_rx.recv();
        pin_mut!(msg_fut);

        match future::select(msg_fut, interrupt).await {
            Either::Left((new_state, _)) => match new_state {
                Ok(new_state) => state = new_state,
                Err(async_channel::RecvError) => {
                    // shutting down
                    return;
                }
            },
            Either::Right((_, _)) => {
                // interrupt!
                irq.assert();

                match state {
                    InterrupterState::Disabled => unreachable!(),
                    InterrupterState::Oneshot { .. } => state = InterrupterState::Disabled,
                    InterrupterState::Repeating {
                        ref mut next,
                        period,
                    } => *next += period,
                }
            }
        }
    }
}

/// Configurable microsecond timer used on the PP5020.
// XXX: check hardware if the timer is incrementing or decrementing
#[derive(Debug)]
pub struct CfgTimer {
    label: &'static str,
    irq: irq::Sender,

    counter: u32,
    unknown_cfg: bool,
    repeat: bool,
    enable: bool,

    val: u32,

    last: Instant,
    last_interrupter_state: Option<InterrupterState>,
    interrupter_tx: async_channel::Sender<InterrupterState>,
}

impl CfgTimer {
    pub fn new(label: &'static str, irq: irq::Sender) -> CfgTimer {
        // TODO: this should probably be bounded, right?
        let (interrupter_tx, interrupter_rx) = async_channel::unbounded();

        std::thread::spawn({
            let irq_clone = irq.clone();
            move || futures_executor::block_on(interrupter_task(irq_clone, interrupter_rx))
        });

        CfgTimer {
            label,
            irq,

            counter: 0,
            unknown_cfg: false,
            repeat: false,
            enable: false,

            val: 0,

            last: Instant::now(),
            last_interrupter_state: None,
            interrupter_tx,
        }
    }

    fn update_regs(&mut self) -> MemResult<()> {
        // XXX: this update code doesn't reference the counter _at all_.
        // As it stands, this code is identical to the regular, non-IRQ usec_timer.
        // Need to experiment with the actual hardware to determine proper behavior...

        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        let elapsed_as_micros = elapsed.as_micros() as u32;
        // Reading the timer value in a tight loop could result in a delta time of 0
        if elapsed_as_micros != 0 {
            self.val = self.val.wrapping_sub(elapsed_as_micros);
            self.last = now;
        }

        Ok(())
    }
}

impl Device for CfgTimer {
    fn kind(&self) -> &'static str {
        "Microsecond Timer (with IRQ config)"
    }

    fn label(&self) -> Option<&'static str> {
        Some(self.label)
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x0 => "Config",
            0x4 => "Val / Clear IRQ",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for CfgTimer {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        self.update_regs()?;

        match offset {
            0x0 => Ok(*0u32
                .set_bits(0..=28, self.counter)
                .set_bit(29, self.unknown_cfg)
                .set_bit(30, self.repeat)
                .set_bit(31, self.enable)),
            0x4 => Ok({
                self.irq.clear();
                self.val
            }),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        self.update_regs()?;

        match offset {
            0x0 => Ok({
                let prev_enable = self.enable;

                self.counter = val.get_bits(0..=28);
                self.unknown_cfg = val.get_bit(29);
                self.repeat = val.get_bit(30);
                self.enable = val.get_bit(31);

                let new_state = {
                    if self.enable && !prev_enable {
                        let period = Duration::from_micros(self.counter as _);
                        Some(if self.repeat {
                            InterrupterState::Repeating {
                                next: Instant::now() + period,
                                period,
                            }
                        } else {
                            InterrupterState::Oneshot {
                                next: Instant::now() + period,
                            }
                        })
                    } else if !self.enable {
                        Some(InterrupterState::Disabled)
                    } else {
                        None
                    }
                };

                if let Some(new_state) = new_state {
                    self.last_interrupter_state = Some(new_state);
                    self.interrupter_tx
                        .try_send(new_state)
                        .map_err(|e| Fatal(format!("couldn't set new timer state: {}", e)))?
                }
            }),
            0x4 => Err(InvalidAccess),
            _ => Err(Unexpected),
        }
    }
}
