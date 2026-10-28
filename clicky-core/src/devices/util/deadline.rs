use std::time::Duration;

use relativity::Instant;

/// Wall-clock deadline
#[derive(Debug, Clone, Copy)]
pub struct Deadline {
    next: Option<Instant>,
    /// Re-arm period
    period: Option<Duration>,
    /// Steps until the next wall-clock check
    countdown: u32,
}

impl Deadline {
    /// Steps between wall-clock checks, so the system loop doesn't pay for
    /// `Instant::now()` on every step.
    const CHECK_INTERVAL: u32 = 256;

    pub fn disabled() -> Deadline {
        Deadline {
            next: None,
            period: None,
            countdown: Self::CHECK_INTERVAL,
        }
    }

    pub fn oneshot(after: Duration) -> Deadline {
        Deadline {
            next: Some(Instant::now() + after),
            ..Deadline::disabled()
        }
    }

    pub fn repeating(period: Duration) -> Deadline {
        Deadline {
            next: Some(Instant::now() + period),
            period: Some(period),
            ..Deadline::disabled()
        }
    }

    pub fn is_armed(&self) -> bool {
        self.next.is_some()
    }

    /// Call once per system step. Returns true once per expiry. A repeating
    /// deadline that fell behind re-arms from now instead of replaying the
    /// missed periods.
    pub fn expired(&mut self) -> bool {
        self.countdown -= 1;
        if self.countdown != 0 {
            return false;
        }
        self.countdown = Self::CHECK_INTERVAL;

        let now = Instant::now();
        let next = match self.next {
            Some(next) if next <= now => next,
            _ => return false,
        };

        self.next = self.period.map(|period| {
            let next = next + period;
            if next >= now {
                return next;
            }

            let behind = now - next;
            if behind > period * 100 {
                warn!("deadline fell {:?} behind, dropping missed periods", behind);
            }
            now + period
        });

        true
    }
}
