//! One cooperative monotonic budget shared by all phases of a finite request.
use crate::domain::{Error, Result};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Deadline {
    end: Instant,
}
impl Deadline {
    pub fn from_instant(end: Instant) -> Self {
        Self { end }
    }
    pub fn from_millis(milliseconds: u64) -> Result<Self> {
        if milliseconds == 0 {
            return Err(Error::new(
                "INVALID_ARGUMENT",
                "Timeout must be positive",
                2,
            ));
        }
        let end = Instant::now()
            .checked_add(Duration::from_millis(milliseconds))
            .ok_or_else(|| Error::new("INVALID_ARGUMENT", "Timeout exceeds supported range", 2))?;
        Ok(Self { end })
    }
    pub fn remaining(self) -> Result<Duration> {
        self.end
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| Error::new("TIMEOUT", "Request deadline expired", 7))
    }
    pub fn check(self) -> Result<()> {
        self.remaining().map(|_| ())
    }
    pub fn instant(self) -> Instant {
        self.end
    }
}
