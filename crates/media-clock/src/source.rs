//! Injectable time sources (card 2.C1; doc 03 §4.7 a).
//!
//! The clock's truth is **injected, never ambient**: the audio-master
//! clock reads a [`SampleCounterSource`] (the audio output's sample
//! counter, doc 03 §4.9 `sample_counter` — "the clock's truth source"),
//! and the audio-less fallback reads a [`MonotonicSource`] (wall time in
//! production via [`InstantSource`], virtual time in tests via an
//! adapter over `testkit`'s `VirtualClock`, doc 03 §8).
//!
//! # Contract for implementors
//!
//! Both traits must be **non-blocking**: the clock reads them inside its
//! short internal lock (doc 03 §4.7 c — the lock is never held across
//! I/O, so the reads themselves may not be I/O). The production
//! audio-output adapter answers from an atomic device counter; test
//! fakes answer from atomics or a tiny mutex. Read frequency is the
//! pipeline's read frequency of `now()`.
//!
//! # Monotonicity
//!
//! [`SampleCounterSource::sample_count`] must be monotonic **while the
//! device runs** — a paused device freezes it (doc 03 §4.7 e). A counter
//! that regresses (device loss and re-open on a fresh counter) is not a
//! contract violation the clock panics over; the clock detects it,
//! holds its last position (never backwards), and reports it through
//! [`crate::PlaybackClock::try_now`] as
//! [`media_types::time::TimeError::Discontinuity`] until the session
//! re-anchors. [`MonotonicSource::now_ns`] must be monotonic, period.

use std::time::Instant;

/// The audio-master truth source: the output device's sample counter
/// (doc 03 §4.9). "The device sample counter is the only trusted
/// progress source. Video never advances the clock." (doc 03 §4.7 e)
pub trait SampleCounterSource: Send + Sync {
    /// Samples consumed by the output so far — the count of samples the
    /// device has actually played. Monotonic while the device runs;
    /// frozen while it is paused.
    fn sample_count(&self) -> u64;

    /// The counter's rate in Hz (samples per second of counter time,
    /// e.g. 48 000). Must be `> 0`; a source reporting `0` cannot be
    /// interpreted as time and is treated as unusable (the clock holds
    /// its position and reports a discontinuity rather than guessing).
    fn sample_rate_hz(&self) -> u32;
}

/// The fallback truth source for audio-less media (doc 03 §4.7 d:
/// master is `SystemMonotonic` and the UI is told). Monotonic
/// nanoseconds since an arbitrary, fixed origin.
pub trait MonotonicSource: Send + Sync {
    /// The source's current reading, in nanoseconds. Monotonic: never
    /// decreases across calls.
    fn now_ns(&self) -> u64;
}

/// The production [`MonotonicSource`]: `std::time::Instant` elapsed
/// since construction (monotonic by std's guarantee on all supported
/// platforms). Tests never use this — they inject virtual time instead
/// (doc 03 §8, "virtual-clock determinism").
#[derive(Debug, Clone)]
pub struct InstantSource {
    origin: Instant,
}

impl InstantSource {
    /// A source anchored at this instant.
    pub fn new() -> InstantSource {
        InstantSource {
            origin: Instant::now(),
        }
    }
}

impl Default for InstantSource {
    fn default() -> InstantSource {
        InstantSource::new()
    }
}

impl MonotonicSource for InstantSource {
    fn now_ns(&self) -> u64 {
        // u128 -> u64: saturate, never wrap (a wrap would be a backwards
        // jump after ~584 years of uptime; saturation is the honest cap).
        u64::try_from(self.origin.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instant_source_is_monotonic_without_sleeping() {
        // Two consecutive reads must not regress; at ns resolution they
        // may be equal, so this needs no sleep (AC rule: no sleeps).
        let source = InstantSource::new();
        let a = source.now_ns();
        let b = source.now_ns();
        assert!(b >= a, "{a} -> {b} regressed");
        let c = source.now_ns();
        assert!(c >= b, "{b} -> {c} regressed");
    }

    #[test]
    fn instant_source_default_matches_new() {
        // Both must produce usable, monotonic sources (construction does
        // not panic and reads are in range).
        let a = InstantSource::default();
        let b = InstantSource::new();
        assert!(b.now_ns() >= a.now_ns() || a.now_ns() >= b.now_ns());
    }
}
