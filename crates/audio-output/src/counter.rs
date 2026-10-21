//! The device sample counter: the clock's truth source (doc 03 §4.9
//! `sample_counter` — "the clock's truth source"; §4.7 e — "the
//! device sample counter is the only trusted progress source. Video
//! never advances the clock.").
//!
//! One atomic word, monotonically advanced by the render callback as
//! the device consumes frames. Units are **sample instants** (one
//! instant = one frame across all channels — the unit
//! [`media_clock::SampleCounterSource`] rate semantics use: 44 100
//! instants at 44 100 Hz is one second, stereo or not).
//!
//! # Pause and device switches
//!
//! Pause freezes the counter (doc 03 §4.7 e: the paused device stops
//! rendering, and only rendering advances the counter — frozen *by
//! construction*, not by a special case). Across device switches the
//! counter **never rewinds** (doc 03 §4.9 e): [`SampleCounter::with_base`]
//! carries the previous device's count as the new counter's origin —
//! the offset-carry invariant, structural and tested.

use std::sync::atomic::{AtomicU64, Ordering};

use media_clock::SampleCounterSource;
use media_types::EngineError;

use crate::error;

/// The atomic device sample counter (an [`SampleCounterSource`]).
#[derive(Debug)]
pub struct SampleCounter {
    count: AtomicU64,
    rate_hz: u32,
}

impl SampleCounter {
    /// A counter at zero, ticking at `rate_hz`.
    ///
    /// # Errors
    /// [`Unsupported`](media_types::ErrorKind::Unsupported) with code
    /// `audio.sample_rate_invalid`: a zero rate cannot be interpreted
    /// as time (media-clock treats it as unusable, doc 03 §4.7).
    pub fn new(rate_hz: u32) -> Result<SampleCounter, EngineError> {
        if rate_hz == 0 {
            return Err(error::unsupported("audio.sample_rate_invalid"));
        }
        Ok(SampleCounter {
            count: AtomicU64::new(0),
            rate_hz,
        })
    }

    /// A counter starting at `base` instants — the device-switch
    /// offset carry (doc 03 §4.9 e: "monotonic and never rewinds
    /// across device switches"). The old device's final count is the
    /// new device's origin.
    ///
    /// # Errors
    /// As [`SampleCounter::new`].
    pub fn with_base(base: u64, rate_hz: u32) -> Result<SampleCounter, EngineError> {
        let counter = SampleCounter::new(rate_hz)?;
        counter.count.store(base, Ordering::Relaxed);
        Ok(counter)
    }

    /// Advance by the instants the device just consumed (the render
    /// callback's one counter operation). Saturating — u64 instants
    /// at any rate outlive every playback session ever started.
    pub fn advance(&self, instants: u64) {
        let _ = self.count.fetch_add(instants, Ordering::Relaxed);
    }

    /// The instants consumed so far (monotonic while running, frozen
    /// while paused — see the module docs).
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// The counter's rate in instants per second.
    pub fn rate_hz(&self) -> u32 {
        self.rate_hz
    }
}

impl SampleCounterSource for SampleCounter {
    fn sample_count(&self) -> u64 {
        self.count()
    }

    fn sample_rate_hz(&self) -> u32 {
        self.rate_hz
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testkit::Must;

    #[test]
    fn zero_rate_is_typed() {
        let err = SampleCounter::new(0).err().must();
        assert_eq!(err.context.code, "audio.sample_rate_invalid");
        let err = SampleCounter::with_base(1000, 0).err().must();
        assert_eq!(err.context.code, "audio.sample_rate_invalid");
    }

    #[test]
    fn counter_is_monotonic_and_rate_carrying() {
        let counter = SampleCounter::new(44_100).must();
        assert_eq!(counter.count(), 0);
        counter.advance(1470);
        counter.advance(1470);
        assert_eq!(counter.count(), 2940);
        // The SampleCounterSource view (what the clock reads).
        let source: &dyn SampleCounterSource = &counter;
        assert_eq!(source.sample_count(), 2940);
        assert_eq!(source.sample_rate_hz(), 44_100);
    }

    #[test]
    fn offset_carry_never_rewinds_across_devices() {
        let old = SampleCounter::new(48_000).must();
        old.advance(48_000 * 60);
        let new = SampleCounter::with_base(old.count(), 48_000).must();
        assert!(new.count() >= old.count());
        new.advance(1);
        assert!(
            new.count() > old.count(),
            "the switch is a freeze, not a rewind"
        );
    }
}
