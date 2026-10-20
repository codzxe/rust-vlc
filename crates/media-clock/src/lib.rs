//! The Lumen media clock: audio-master playback timing (card 2.C1,
//! doc 03 §4.7; doc 01 §2.2 registry row and §1.4 invariant 8 "one clock").
//!
//! # What this crate is
//!
//! The single source of playback time for the whole pipeline. Audio is
//! master by default (MP §3): [`MediaClock`] derives `now()` from the
//! audio output's **sample counter** — the count of samples consumed by
//! the output — never from a wall clock. When a medium has no audio (or
//! unsupported audio), the clock falls back **explicitly** to a monotonic
//! time source ([`ClockMaster::SystemMonotonic`]) so the UI can tell the
//! truth rather than trust a silent approximation (doc 03 §4.7 d).
//!
//! # The injected truth
//!
//! Time is **injected, never ambient** (card 2.C1): production wires the
//! audio-output adapter or [`InstantSource`] into the clock; tests wire
//! virtual sources built on `testkit`'s `VirtualClock`, so every
//! acceptance test advances time deterministically — no sleeps (doc 03
//! §8). See [`source`].
//!
//! # Time vocabulary
//!
//! All time concepts are [`media-types`]' (doc 01 §3): positions are
//! [`Timestamp`]s in the canonical nanosecond base [`NANO_BASE`] (the
//! analog of doc 03 §3's `MediaTime`), playback rates are exact
//! rationals ([`Rate`], doc 01 §3.1 — `5/4` = 1.25×, no float drift),
//! and failures are the time taxonomy [`TimeError`]: overflow, and
//! discontinuity — conversion failure is data, never a panic (MP §8).
//!
//! # Concurrency (doc 03 §4.7 c)
//!
//! No dedicated thread. The audio pump's counter updates and every
//! reader's `now()` go through one short internal lock whose critical
//! sections are pure arithmetic — never held across I/O. Sources must be
//! non-blocking ([`source`] contract). The lock-free / seqlock read path
//! is Phase-3 hardening; v0's short lock satisfies the "never held
//! across I/O" clause and keeps all arithmetic checked and panic-free.
//!
//! # Semantics summary (doc 03 §4.7 e)
//!
//! - `now() = anchor + elapsed(counter) × rate` within one anchor
//!   epoch: monotonic — never backwards, equal at worst.
//! - [`MediaClock::reanchor`] (open, seek, discontinuity) starts a new
//!   epoch; a jump there is expected, and reported through the epoch in
//!   [`ClockSnapshot`].
//! - Pause freezes the position; resume continues from the same media
//!   time with no discontinuity — even if the underlying source kept
//!   running while paused.
//! - Drift correction is a **bounded** rate adjustment
//!   ([`DriftCorrection`], ±1000 ppm); the limiter is the invariant,
//!   the number is a Phase-3 measurement away (doc 03 §4.7 e).
//!
//! # Example
//!
//! ```
//! use media_clock::{MediaClock, PlaybackClock, SampleCounterSource};
//! use media_types::time::{Rate, Timestamp};
//! use std::sync::atomic::{AtomicU64, Ordering};
//! use std::sync::Arc;
//!
//! // A minimal sample-counter source: the audio-output adapter in
//! // production, a virtual device in tests (doc 03 §4.9).
//! struct Counter(AtomicU64);
//! impl SampleCounterSource for Counter {
//!     fn sample_count(&self) -> u64 {
//!         self.0.load(Ordering::Relaxed)
//!     }
//!     fn sample_rate_hz(&self) -> u32 {
//!         48_000
//!     }
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let zero = Timestamp::from_parts(0, 1, 1_000_000_000)?;
//! let counter = Arc::new(Counter(AtomicU64::new(0)));
//! let clock = MediaClock::audio_master(counter.clone(), zero, Rate::ONE)?;
//!
//! // The pump consumes one second of audio; the clock follows.
//! let _ = counter.0.fetch_add(48_000, Ordering::Relaxed);
//! assert_eq!(clock.now().ticks, 1_000_000_000);
//!
//! // Pause freezes the position even if the source keeps running...
//! clock.pause();
//! let _ = counter.0.fetch_add(48_000, Ordering::Relaxed);
//! assert_eq!(clock.now().ticks, 1_000_000_000);
//! // ...and resume continues with no discontinuity.
//! clock.resume();
//! assert_eq!(clock.now().ticks, 1_000_000_000);
//! # Ok(())
//! # }
//! ```
//!
//! # Never
//!
//! I/O, device APIs, UI, threads of its own (doc 01 §2.2 d). The clock
//! observes; it never drives the audio device.

pub mod clock;
pub mod drift;
pub mod mapping;
pub mod source;

pub use clock::{ClockAnchor, ClockMaster, ClockSnapshot, ClockState, MediaClock, PlaybackClock};
pub use drift::{DriftCorrection, MAX_CORRECTION_PPM};
pub use mapping::{
    NANO_BASE, map_samples_to_timestamp, map_timestamp_to_samples, nanos_to_timestamp,
    sample_delta_to_media_nanos, samples_until_end, timestamp_to_nanos,
};
pub use source::{InstantSource, MonotonicSource, SampleCounterSource};

use media_types::time::Rate;

/// Defensive re-validation of a [`Rate`] at the clock's mutation
/// boundaries: [`Rate`]'s pub fields allow constructing zero-component
/// values that bypass [`Rate::new`]. Mirrors `Rate::new` exactly
/// (rejects zero components; reduces) so stored rates are always sound.
///
/// # Errors
/// [`media_types::EngineError`] with kind `Unsupported` and stage
/// `Control` — `Rate::new`'s error.
pub(crate) fn validate_rate(rate: Rate) -> Result<Rate, media_types::EngineError> {
    Rate::new(rate.num, rate.den)
}
