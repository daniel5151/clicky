use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel as chan;
use log::Level::*;

use crate::devices::{Device, Interrupt, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

const KHZ: usize = 32; // XXX: pulled out of my ass lol

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    FreeRunning = 0,
    Periodic = 1,
}

enum InterrupterMsg {
    Enabled { next: Instant, period: Duration },
    Disabled,
}

fn spawn_interrupter_thread<I: Interrupt>(
    label: &'static str,
    interrupt_bus: chan::Sender<(I, bool)>,
    interrupt: I,
) -> (JoinHandle<()>, chan::Sender<InterrupterMsg>) {
    let (tx, rx) = chan::unbounded::<InterrupterMsg>();
    let thread = move || {
        let mut next: Option<Instant> = None;
        let mut period = Default::default();
        loop {
            let timeout = match next {
                Some(next) => next.saturating_duration_since(Instant::now()),
                // XXX: Not technically correct, but this is a long enough time
                None => Duration::from_secs(std::u32::MAX as _),
            };

            match rx.recv_timeout(timeout) {
                Ok(InterrupterMsg::Enabled {
                    next: new_next,
                    period: new_period,
                }) => {
                    next = Some(new_next);
                    period = new_period;
                }
                Ok(InterrupterMsg::Disabled) => next = None,
                Err(chan::RecvTimeoutError::Disconnected) => {
                    // Sender exited
                    return;
                }
                Err(chan::RecvTimeoutError::Timeout) => {
                    // Interrupt!
                    interrupt_bus.send((interrupt, true)).unwrap();
                    next = Some(
                        next.expect("Impossible: We timed out with an infinite timeout") + period,
                    );
                }
            }
        }
    };

    let handle = thread::Builder::new()
        .name(format!("{} | IrqTimer Interrupter", label))
        .spawn(thread)
        .unwrap();

    (handle, tx)
}

/// A configurable 32bit timer.
#[derive(Debug)]
pub struct IrqTimer<I: Interrupt> {
    label: &'static str,
    // registers
    val: u32,

    loadval: Option<u32>,
    enabled: bool,
    mode: Mode,
    unknown_cfg: bool, // it is a mystery 👻

    // implementation details
    last_time: Instant,
    microticks: u32,

    interrupt_bus: chan::Sender<(I, bool)>,
    interrupt: I,

    interrupter_tx: chan::Sender<InterrupterMsg>,
}

impl<I: Interrupt> IrqTimer<I> {
    /// Create a new IrqTimer
    pub fn new(
        label: &'static str,
        interrupt_bus: chan::Sender<(I, bool)>,
        interrupt: I,
    ) -> IrqTimer<I> {
        let (_, interrupter_tx) = spawn_interrupter_thread(label, interrupt_bus.clone(), interrupt);
        IrqTimer {
            label,
            loadval: None,
            val: 0,
            unknown_cfg: false,
            enabled: false,
            mode: Mode::FreeRunning,
            last_time: Instant::now(),
            microticks: 0,

            interrupt,
            interrupter_tx,
            interrupt_bus,
        }
    }

    /// Lazily update the registers on read / write.
    fn update_regs(&mut self) -> MemResult<()> {
        // calculate the time delta
        let now = Instant::now();
        let dt = now.duration_since(self.last_time).as_nanos() as u64;
        self.last_time = now;

        if !self.enabled {
            return Ok(());
        }

        // calculate number of ticks the timer should decrement by
        let microticks = dt * KHZ as u64 + self.microticks as u64;
        let ticks = (microticks / 1_000_000) as u32;
        self.microticks = (microticks % 1_000_000) as u32;

        match self.mode {
            Mode::FreeRunning => {
                self.val = self.val.wrapping_sub(ticks);
            }
            Mode::Periodic => {
                let loadval = match self.loadval {
                    Some(v) => v,
                    None => {
                        return Err(ContractViolation {
                            msg: "Periodic mode enabled before setting a Load value".to_string(),
                            severity: Error,
                            stub_val: None,
                        })
                    }
                };
                self.val = if loadval == 0 {
                    0
                } else if self.val < ticks {
                    let remaining_ticks = ticks - self.val;
                    loadval - (remaining_ticks % loadval)
                } else {
                    self.val - ticks
                }
            }
        }

        Ok(())
    }
}

impl<I: Interrupt> Device for IrqTimer<I> {
    fn kind(&self) -> &'static str {
        "IrqTimer"
    }

    fn label(&self) -> Option<&'static str> {
        Some(self.label)
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "Config",
            0x04 => "Value",
            _ => return Probe::Unmapped,
        };
        Probe::Register(reg)
    }
}

impl<I: Interrupt> Memory for IrqTimer<I> {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        self.update_regs()?;

        match offset {
            0x00 => {
                let loadval = match self.loadval {
                    Some(v) => v,
                    None => {
                        return Err(ContractViolation {
                            msg: "cannot read Load value before it's been set".into(),
                            severity: Error,
                            stub_val: None,
                        })
                    }
                };

                let val = (loadval & 0x1fff_ffff)
                    | (self.enabled as u32) << 31
                    | (self.mode as u32) << 30
                    | (self.unknown_cfg as u32) << 29;

                Ok(val)
            }
            0x04 => {
                // reading clears the interrupt
                self.interrupt_bus.send((self.interrupt, false)).unwrap();
                Ok(self.val)
            }
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        self.update_regs()?;

        match offset {
            0x00 => {
                let previous_enabled = self.enabled;

                self.loadval = Some(val & 0x1fff_ffff);
                self.enabled = val & (1 << 31) != 0;
                self.mode = match val & (1 << 30) != 0 {
                    true => Mode::Periodic,
                    false => Mode::FreeRunning,
                };
                self.unknown_cfg = val & (1 << 29) != 0;

                self.val = val;

                if self.enabled && !previous_enabled {
                    self.microticks = 0;

                    if self.mode == Mode::Periodic {
                        let loadval = match self.loadval {
                            Some(v) => v,
                            None => {
                                return Err(ContractViolation {
                                    msg: "Periodic mode enabled before setting a Load value".into(),
                                    severity: Error,
                                    stub_val: None,
                                })
                            }
                        };

                        let period =
                            Duration::from_nanos((loadval as u64) * 1_000_000 / KHZ as u64);
                        self.interrupter_tx
                            .send(InterrupterMsg::Enabled {
                                next: Instant::now() + period,
                                period,
                            })
                            .unwrap();
                    }
                }

                if !self.enabled {
                    self.loadval = None;
                    self.interrupter_tx.send(InterrupterMsg::Disabled).unwrap();
                }

                Ok(())
            }
            0x04 => Err(InvalidAccess),
            _ => Err(Unexpected),
        }
    }
}
