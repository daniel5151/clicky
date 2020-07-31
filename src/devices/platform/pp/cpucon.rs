use crate::devices::prelude::*;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub use super::common::CpuId;

#[allow(dead_code)]
mod flags {
    type Range = std::ops::RangeInclusive<usize>;

    /* Control flags, can be ORed together */
    pub const FLOW_MASK: Range = 29..=31;

    /// Sleep until an interrupt occurs
    pub const PROC_SLEEP: usize = 31;
    /// Sleep until end of countdown
    pub const PROC_WAIT_CNT: usize = 30;
    /// Fire interrupt on wake-up. Auto-clears.
    pub const PROC_WAKE_INT: usize = 29;

    /* Counter source, select one */
    // !! not sure what happens if multiple are set !!

    /// Counter Source - Clock cycles
    pub const PROC_CNT_CLKS: usize = 27;
    /// Counter Source - Microseconds
    pub const PROC_CNT_USEC: usize = 25;
    /// Counter Source - Milliseconds
    pub const PROC_CNT_MSEC: usize = 24;
    /// Counter Source - Seconds. Works on PP5022+ only!
    pub const PROC_CNT_SEC: usize = 23;

    pub const PROC_CNT_MASK: Range = 23..=27;
    pub const COUNTER: Range = 0..=7;
}

enum CounterSource {
    Sysclock,
    Micros,
    Millis,
    Sec,
}

impl CounterSource {
    fn into_duration(self, counter: u8) -> Duration {
        match self {
            // XXX: sysclock duration is wildly incorrect lol
            CounterSource::Sysclock => Duration::from_nanos(counter as _),
            CounterSource::Micros => Duration::from_micros(counter as _),
            CounterSource::Millis => Duration::from_millis(counter as _),
            CounterSource::Sec => Duration::from_secs(counter as _),
        }
    }
}

/// PP5020 CPU controller
#[derive(Debug)]
pub struct CpuCon {
    cpuctl: Arc<AtomicU32>,
    copctl: Arc<AtomicU32>,
}

impl CpuCon {
    pub fn new() -> CpuCon {
        CpuCon {
            cpuctl: Arc::new(0x0000_0000.into()),
            copctl: Arc::new(0x0000_0000.into()),
        }
    }

    pub fn is_cpu_running(&mut self, cpu: CpuId) -> bool {
        let cpuctl = match cpu {
            CpuId::Cpu => &self.cpuctl,
            CpuId::Cop => &self.copctl,
        };

        // this might not be it chief
        cpuctl.load(Ordering::SeqCst).get_bits(flags::FLOW_MASK) == 0
    }

    pub fn wake_on_interrupt(&mut self, cpu: CpuId) {
        let cpuctl = match cpu {
            CpuId::Cpu => &self.cpuctl,
            CpuId::Cop => &self.copctl,
        };

        // this might not be it chief
        let _ = cpuctl.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |mut reg| {
            if reg.get_bit(flags::PROC_SLEEP) {
                reg = *reg.set_bits(flags::FLOW_MASK, 0);
            }
            Some(reg)
        });
    }

    fn on_update_cpuctl(&self, cpu: CpuId, val: u32) -> MemResult<()> {
        if val.get_bit(flags::PROC_WAIT_CNT) {
            match val.get_bits(flags::PROC_CNT_MASK).count_ones() {
                0 => return Ok(()), // TODO: double check if this is a synonym for sleep?
                1 => {}             // expected case
                _ => {
                    return Err(ContractViolation {
                        msg: "set more than one counter source".into(),
                        severity: Error,
                        stub_val: None,
                    });
                }
            }
            let source = match val {
                _ if val.get_bit(flags::PROC_CNT_CLKS) => CounterSource::Sysclock,
                _ if val.get_bit(flags::PROC_CNT_USEC) => CounterSource::Micros,
                _ if val.get_bit(flags::PROC_CNT_MSEC) => CounterSource::Millis,
                _ if val.get_bit(flags::PROC_CNT_SEC) => CounterSource::Sec,
                _ => {
                    return Err(ContractViolation {
                        msg: "set invalid counter source (bit 26)".into(),
                        severity: Error,
                        stub_val: None,
                    })
                }
            };

            let duration = source.into_duration(val.get_bits(flags::COUNTER) as u8);

            // XXX: use a proper async executor instead of spawning a thread!
            std::thread::spawn({
                let reg = Arc::clone(match cpu {
                    CpuId::Cpu => &self.cpuctl,
                    CpuId::Cop => &self.copctl,
                });
                // create timer outside of the task for slightly improved accuracy
                let timer = async_timer::new_timer(duration);

                move || {
                    futures_executor::block_on(async {
                        timer.await;
                        // TODO: check if flags::PROC_WAKE_INT is set, and fire an interrupt
                        reg.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |mut reg| {
                            let reg = *reg.set_bits(flags::FLOW_MASK, 0);
                            Some(reg)
                        })
                        .unwrap();
                    })
                }
            });
        }

        if val.get_bit(flags::PROC_WAKE_INT) {
            return Err(Fatal(format!(
                "unimplemented: 'Fire interrupt on wake-up' for {:?}",
                cpu
            )));
        }

        // TODO: check flow bits and setup timers to wake CPU
        Ok(())
    }
}

impl Device for CpuCon {
    fn kind(&self) -> &'static str {
        "System Controller Block"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x0 => "CPU Control",
            0x4 => "COP Control",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for CpuCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0 => Ok(self.cpuctl.load(Ordering::SeqCst)),
            0x4 => Ok(self.copctl.load(Ordering::SeqCst)),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x0 => Ok({
                self.cpuctl.store(val, Ordering::SeqCst);
                self.on_update_cpuctl(CpuId::Cpu, val)?;
            }),
            0x4 => Ok({
                self.copctl.store(val, Ordering::SeqCst);
                self.on_update_cpuctl(CpuId::Cop, val)?;
            }),
            _ => Err(Unexpected),
        }
    }
}
