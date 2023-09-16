#![warn(clippy::pedantic, clippy::nursery)]

use nix::{
    sys::time::TimeSpec,
    time::{clock_gettime, ClockId},
};

pub const CURRENT_GENERATION: u64 = 0;

pub const GENERATION_BITS: usize = 6;
pub const TIME_BITS: usize = 42;
pub const COUNTER_BITS: usize = 16;

/// Difference between 2020-01-01T00:00:00Z and the Unix epoch
pub const SECOND_EPOCH: u64 = 1_577_836_800;

// Just a basic, obvious, compile-time check.
#[allow(clippy::assertions_on_constants)]
const _: () = assert!(GENERATION_BITS + TIME_BITS + COUNTER_BITS == 64);

pub const TIME_SHIFT_BITS: usize = 10;
pub const SEC_IN_NANOS: u64 = 1_000_000_000;
pub const TIME_MASK: u64 = ((1 << TIME_BITS) - 1) << COUNTER_BITS;
pub const GENERATION_IN_POSITION: u64 = CURRENT_GENERATION << (TIME_BITS + COUNTER_BITS);

#[derive(Debug)]
pub enum Error {
    NixError(nix::errno::Errno),
    NegativeTimeSpec(TimeSpec),
}

impl From<nix::errno::Errno> for Error {
    fn from(err: nix::errno::Errno) -> Self {
        Self::NixError(err)
    }
}

pub type Result<T> = core::result::Result<T, Error>;

#[must_use]
pub const fn stamp_time(secs: u64, nanos: u64) -> u64 {
    let shifted_secs = secs << TIME_SHIFT_BITS;
    let shifted_nanos = nanos << TIME_SHIFT_BITS;
    let in_position = (shifted_secs + (shifted_nanos - SEC_IN_NANOS)) << COUNTER_BITS;
    GENERATION_IN_POSITION | (in_position & TIME_MASK)
}

pub struct Stamp(u64);

pub struct Inst {
    secs: u64,
    nanos: u64,
}

impl Inst {
    #[must_use]
    pub const fn new(secs: u64, nanos: u64) -> Self {
        Self { secs, nanos }
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self::new(0, 0)
    }

    #[must_use]
    pub const fn stamp(self) -> Stamp {
        Stamp(stamp_time(self.secs, self.nanos))
    }
}

pub trait TimeSource {
    fn tick() -> Result<Inst>;

    /// Gets a valid Inst or PANIC!
    fn must_tick() -> Inst {
        Self::tick().unwrap()
    }
}

/// MonotonicClock clocks monotonically, yo.
pub struct MonotonicClock {
    /// The Inst at which this clack was initialized.
    init: Inst,
}

impl MonotonicClock {
    pub fn new() -> Self {
        Self::try_new().unwrap()
    }

    pub fn try_new() -> Result<Self> {
        match Self::tick() {
            Ok(init) => Ok(Self { init }),
            Err(err) => Err(err),
        }
    }
}

impl TimeSource for MonotonicClock {
    fn tick() -> Result<Inst> {
        let tspec = clock_gettime(ClockId::CLOCK_BOOTTIME)?;
        let secs = tspec.tv_sec();
        let nanos = tspec.tv_nsec();
        if secs < 0 || nanos < 0 {
            return Err(Error::NegativeTimeSpec(tspec));
        }
        #[allow(clippy::cast_sign_loss)]
        Ok(Inst::new(secs as u64, nanos as u64))
    }
}

pub struct AtomicClock<T> {
    source: T,
}

impl AtomicClock<MonotonicClock> {
    #[must_use]
    pub fn new() -> Self {
        Self::try_new().unwrap()
    }

    #[must_use]
    pub fn try_new() -> Result<Self> {
        match MonotonicClock::try_new() {
            Ok(source) => Ok(Self { source }),
            Err(err) => Err(err),
        }
    }
}

impl<T: TimeSource> AtomicClock<T> {
    pub const fn with_source(source: T) -> Self {
        Self { source }
    }

    pub fn now(&self) -> Inst {
        self.try_now().unwrap()
    }

    pub fn try_now(&self) -> Result<Inst> {
        T::tick()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
