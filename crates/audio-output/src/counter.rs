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
use crate::render::RenderStats;

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
    /// callback's one counter operation). Wrapping — `fetch_add` wraps
    /// on u64 overflow, which is unreachable at audio rates (2^64
    /// instants at 48 kHz is on the order of 12 Myr), so u64 instants
    /// at any rate outlive every playback session ever started
    /// (W08-A01 F-001: the old doc said "Saturating"; the mechanism is
    /// the width, not saturation).
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

/// The sink's discard accounting (FX3-A05, the FX4 end-to-end seam:
/// the audit could prove "decodes into null sinks" but nothing
/// observable flowed — these counts are the observable truth). One
/// shared ledger per session, recorded **driver-side after the
/// real-time callback returns** (capture's discipline — the
/// no-blocking contract's rules never govern it), from the
/// callback's own [`RenderStats`] ledger, so the two accounts cannot
/// disagree.
///
/// Units follow the crate's counter conventions: `frames` are whole
/// sample-frames (the [`SampleCounter`] instants unit — data +
/// silence; for a null session opened at zero base, `frames` and the
/// counter's count are the same consumption), `samples` are f32
/// slots (`frames × channels`), `bytes` are interleaved f32 LE
/// (`samples × 4`). `silence_frames` isolates the underrun-fill
/// subset, so `data_frames` is the exact data count — the
/// null-backend audit's F-002 lesson ("consumed − silence") made a
/// field instead of an invariant comment.
///
/// All single-word `Relaxed` atomics, wrapping on u64 overflow —
/// unreachable at audio rates (2^64 bytes ≈ 18 exabytes) —
/// bookkeeping only, never ordering (the `written` counter's
/// discipline). Monotonic while the device runs, frozen while paused
/// (a paused render consumes nothing and records nothing).
#[derive(Debug, Default)]
pub struct SinkStats {
    /// Render calls served (device buffers pulled).
    buffers: AtomicU64,
    /// Whole sample-frames consumed (data + silence).
    frames: AtomicU64,
    /// f32 sample slots consumed (`frames × channels`).
    samples: AtomicU64,
    /// Bytes consumed (interleaved f32 LE, `samples × 4`).
    bytes: AtomicU64,
    /// Frames of underrun silence inside `frames` (data = frames − this).
    silence_frames: AtomicU64,
}

impl SinkStats {
    /// Record one served render call (the null driver's post-callback
    /// bookkeeping — see the type docs). Crate-private: the ledger is
    /// written by the driver halves only; everyone else reads.
    pub(crate) fn record(&self, call: &RenderStats, channels: usize) {
        // Wrapping fetch_add (the type docs): the u64 scale makes
        // overflow unreachable at audio rates.
        let _ = self.buffers.fetch_add(1, Ordering::Relaxed);
        let _ = self
            .frames
            .fetch_add(call.frames_requested as u64, Ordering::Relaxed);
        let samples = call.frames_requested as u64 * channels as u64;
        let _ = self.samples.fetch_add(samples, Ordering::Relaxed);
        let _ = self.bytes.fetch_add(samples * 4, Ordering::Relaxed);
        let _ = self
            .silence_frames
            .fetch_add(call.silence_filled as u64, Ordering::Relaxed);
    }

    /// Render calls served so far (monotonic).
    pub fn buffers(&self) -> u64 {
        self.buffers.load(Ordering::Relaxed)
    }

    /// Whole sample-frames consumed so far (data + silence — the
    /// [`SampleCounter`] instants unit).
    pub fn frames(&self) -> u64 {
        self.frames.load(Ordering::Relaxed)
    }

    /// f32 sample slots consumed so far (`frames × channels`).
    pub fn samples(&self) -> u64 {
        self.samples.load(Ordering::Relaxed)
    }

    /// Bytes consumed so far (interleaved f32 LE, `samples × 4`) —
    /// the discard's byte total.
    pub fn bytes(&self) -> u64 {
        self.bytes.load(Ordering::Relaxed)
    }

    /// Frames of underrun silence inside `frames` so far.
    pub fn silence_frames(&self) -> u64 {
        self.silence_frames.load(Ordering::Relaxed)
    }

    /// The data subset of `frames` (`frames − silence_frames`,
    /// saturating) — the exact data consumed once underruns exist.
    pub fn data_frames(&self) -> u64 {
        self.frames().saturating_sub(self.silence_frames())
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
